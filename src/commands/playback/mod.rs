use super::*;

pub use self::types::*;

mod types;

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

    trace!("acquiring queue lock");

    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.write().unwrap();

    trace!("queue lock acquired");

    play_queue.queue.push_back(PlayArgs{
        initiator: msg.author.name.clone(),
        url: url.to_owned(),
        sender_channel: msg.channel_id,
    });

    Ok(())
}

command!(play(ctx, msg, args) {
    if args.len() == 0 {
        _resume(ctx, msg)?;
        return Ok(());
    }

    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            send(msg.channel_id, "BAD LINK", msg.tts)?;
            return Ok(());
        }
    };

    _play(ctx, msg, &url)?;
});

command!(pause(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

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

    if !playing {
        done()?;
        return Ok(());
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().pause();
    }
});

command!(resume(ctx, msg) {
    _resume(ctx, msg)?;
});

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

command!(skip(ctx, _msg) {
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
});

command!(die(ctx, msg) {
    let data = ctx.data.lock();

    let mut mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let mut queue_lock = data.get::<PlayQueue>().cloned().unwrap();

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
});

command!(list(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.read().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    match play_queue.playing {
        Some(ref info) => {
            let audio = info.audio.lock();
            let status = if audio.playing { "playing" } else { "paused:" };
            send(msg.channel_id, &format!("Currently {} `{}` ({})", status, info.init_args.url, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            send(msg.channel_id, "Nothing is playing you meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.queue.iter().for_each(|info| {
        channel.say(&format!("`{}` ({})", info.url, info.initiator)).unwrap();
    });
});
