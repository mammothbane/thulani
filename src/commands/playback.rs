use either::{Left, Right};
use log::{
    debug,
    error,
    info,
    warn,
};
use serenity::{
    framework::standard::{
        Args,
        CommandResult,
        macros::{command, group},
    },
    model::channel::Message,
    prelude::*,
};

use crate::{
    audio::{
        parse_times,
        PlayArgs,
        PlayQueue,
        VoiceManager,
    },
    TARGET_GUILD_ID,
    util::CtxExt,
};

group!({
    name: "playback",
    options: {
        only_in: "guild",
    },
    commands: [
        skip,
        pause,
        resume,
        list,
        die,
        mute,
        unmute,
        play,
        volume,
    ],
});

pub fn _play(ctx: &Context, msg: &Message, url: &str) -> CommandResult {
    use url::{Url, Host};

    debug!("playing '{}'", url);
    if !url.starts_with("http") {
        warn!("got bad url argument to play: {}", url);
        ctx.send(msg.channel_id, "bAD LiNk", msg.tts)?;
        return Ok(());
    }

    let url = match Url::parse(url) {
        Err(e) => {
            error!("bad url: {}", e);
            return ctx.send(msg.channel_id, "INVALID URL", msg.tts);
        },
        Ok(u) => u,
    };

    let host = url.host().and_then(|u| match u {
        Host::Domain(h) => Some(h.to_owned()),
        _ => None,
    });

    if host.map(|h| h.to_lowercase().contains("imgur")).unwrap_or(false) {
        info!("detected imgur link");

        if msg.author.id.0 == 106160362109272064 {
            ctx.send(msg.channel_id, "fuck you conway", true)?;
        } else {
            ctx.send(msg.channel_id, "IMGUR IS BAD, YOU TRASH CAN MAN", msg.tts)?;
        }

        return Ok(());
    }

    let (start, end) = parse_times(&msg.content);

    let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.write().unwrap();

    play_queue.general_queue.push_back(PlayArgs{
        initiator: msg.author.name.clone(),
        data: Left(url.into_string()),
        sender_channel: msg.channel_id,
        start,
        end,
    });

    Ok(())
}

#[command]
pub fn play(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() == 0 {
        return _resume(ctx, msg);
    }

    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(e) => {
            error!("unable to parse url from args: {}", e);
            return ctx.send(msg.channel_id, "BAD LINK", msg.tts);
        },
    };

    _play(ctx, msg, &url)
}

#[command]
pub fn pause(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
    let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();

    let done = || ctx.send(msg.channel_id, "r u srs", msg.tts);
    let playing = {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => return done(),
        };

        let audio = current_item.audio.lock();
        audio.playing
    };

    if !playing {
        return done();
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().pause();

        info!("paused playback");
    }

    Ok(())
}

#[command]
#[aliases("continue")]
pub fn resume(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
    _resume(ctx, msg)
}

fn _resume(ctx: &mut Context, msg: &Message) -> CommandResult {
    let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();

    let done = || ctx.send(msg.channel_id, "r u srs", msg.tts);
    let playing = {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => {
                done()?;
                return Ok(());
            },
        };

        let audio = current_item.audio.lock();
        audio.playing
    };

    if playing {
        done()?;
        debug!("attempted to resume playback while sound was already playing");
        return Ok(());
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().play();
        info!("playback resumed");
    }

    Ok(())
}

#[command]
#[aliases("next")]
pub fn skip(ctx: &mut Context, _msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.write();

    let mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        let mut play_queue = queue_lock.write().unwrap();
        play_queue.playing = None;
        info!("skipped currently-playing audio");
    } else {
        debug!("got skip with no handler attached");
    }

    Ok(())
}

#[command]
#[aliases("sudoku", "fuckoff", "stop")]
pub fn die(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
    let data = ctx.data.write();

    let mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    {
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.playing = None;
        play_queue.general_queue.clear();
        play_queue.meme_queue.clear();
    }

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        info!("killing playback");
        handler.stop();
        handler.leave();
    } else {
        ctx.send(msg.channel_id, "YOU die", msg.tts)?;
        debug!("got die with no handler attached");
    }

    Ok(())
}

#[command]
#[aliases("queue")]
pub fn list(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
    let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
    let play_queue = queue_lock.read().unwrap();

    let channel_tmp = msg.channel(ctx).unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    info!("listing queue");
    match play_queue.playing {
        Some(ref info) => {
            let audio = info.audio.lock();
            let status = if audio.playing { "playing" } else { "paused:" };

            let playing_info = match info.init_args.data {
                Left(ref url) => format!(" `{}`", url),
                Right(_) => "memeing".to_owned(),
            };

            ctx.send(msg.channel_id, &format!("Currently {} {} ({})", status, playing_info, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            ctx.send(msg.channel_id, "Nothing is playing you meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.meme_queue.iter()
        .chain(play_queue.general_queue.iter())
        .for_each(|info| {
            let playing_info = match info.data {
                Left(ref url) => format!("`{}`", url),
                Right(_) => "meme".to_owned(),
            };

            let _ = channel.say(ctx, &format!("{} ({})", playing_info, info.initiator));
        });

    Ok(())
}
