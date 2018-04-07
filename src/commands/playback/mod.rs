use either::{Left, Right};
use serenity::voice::{LockedAudio, ytdl};

use super::*;
pub use self::types::*;
use serenity::framework::standard::Args;

mod types;

pub trait CtxExt {
    fn currently_playing(&self) -> bool;
    fn users_listening(&self) -> Result<bool>;
}

impl CtxExt for Context {
    fn currently_playing(&self) -> bool {
        let queue_lock = self.data.lock().get::<PlayQueue>().cloned().unwrap();
        let play_queue = queue_lock.read().unwrap();
        play_queue.playing.is_none()
    }

    fn users_listening(&self) -> Result<bool> {
        let channel_id = ChannelId(must_env_lookup::<u64>("VOICE_CHANNEL"));
        let channel = channel_id.get()?;
        let res = channel.guild()
            .and_then(|ch| ch.read().guild())
            .map(|g| (&g.read().voice_states)
                .into_iter()
                .any(|(_, state)| state.channel_id == Some(channel_id)))
            .unwrap_or(false);

        Ok(res)
    }
}

pub fn _play(ctx: &Context, msg: &Message, url: &str) -> Result<()> {
    debug!("playing '{}'", url);
    if !url.starts_with("http") {
        send(msg.channel_id, "bAD LiNk", msg.tts)?;
        return Ok(());
    }

    if url.contains("imgur") {
        send(msg.channel_id, "IMGUR IS BAD, YOU TRASH CAN MAN", msg.tts)?;
        return Ok(());
    }

    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.write().unwrap();

    play_queue.queue.push_back(PlayArgs{
        initiator: msg.author.name.clone(),
        data: Left(url.to_owned()),
        sender_channel: msg.channel_id,
    });

    Ok(())
}

pub fn play(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    if args.len() == 0 {
        return _resume(ctx, msg);
    }

    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => return send(msg.channel_id, "BAD LINK", msg.tts),
    };

    _play(ctx, msg, &url)
}

pub fn pause(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

    let done = || send(msg.channel_id, "r u srs", msg.tts);
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
    }

    Ok(())
}

pub fn resume(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    _resume(ctx, msg)
}

fn _resume(ctx: &mut Context, msg: &Message) -> Result<()> {
    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

    let done = || send(msg.channel_id, "r u srs", msg.tts);
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
        return Ok(());
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().play();
    }

    Ok(())
}

pub fn skip(ctx: &mut Context, _msg: &Message, _args: Args) -> Result<()> {
    let data = ctx.data.lock();

    let mut mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let mut queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        let mut play_queue = queue_lock.write().unwrap();
        play_queue.playing = None;
    } else {
        debug!("got skip with no handler attached");
    }

    Ok(())
}

pub fn die(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let data = ctx.data.lock();

    let mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    {
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.playing = None;
        play_queue.queue.clear();
    }

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        handler.leave();
    } else {
        send(msg.channel_id, "YOU die", msg.tts)?;
        debug!("got die with no handler attached");
    }

    Ok(())
}

pub fn list(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let play_queue = queue_lock.read().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    match play_queue.playing {
        Some(ref info) => {
            let audio = info.audio.lock();
            let status = if audio.playing { "playing" } else { "paused:" };

            let playing_info = match info.init_args.data {
                Left(ref url) => format!(" `{}`", url),
                Right(_) => "memeing".to_owned(),
            };

            send(msg.channel_id, &format!("Currently {} {} ({})", status, playing_info, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            send(msg.channel_id, "Nothing is playing you meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.queue.iter()
        .for_each(|info| {
            let playing_info = match info.data {
                Left(ref url) => format!("`{}`", url),
                Right(_) => "meme".to_owned(),
            };

            let _ = channel.say(&format!("{} ({})", playing_info, info.initiator));
        });

    Ok(())
}
