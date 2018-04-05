use super::*;

pub const DEFAULT_VOLUME: f32 = 0.05;

command!(mute(ctx, _msg) {
    let mut mgr_lock = ctx.data.lock().get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    manager.get_mut(*TARGET_GUILD_ID)
        .map(|handler| {
            if handler.self_mute {
                trace!("Already muted.")
            } else {
                handler.mute(true);
                trace!("Muted");
            }
        });
});

command!(unmute(ctx, msg) {
    let mut mgr_lock = ctx.data.lock().get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    manager.get_mut(*TARGET_GUILD_ID)
        .map(|handler| {
            if !handler.self_mute {
                trace!("Already unmuted.")
            } else {
                handler.mute(false);
                trace!("Unmuted");
                let _ = send(msg.channel_id, "REEEEEEEEEEEEEE", msg.tts);
            }
        });
});

command!(volume(ctx, msg, args) {
    if args.len() == 0 {
        let vol = {
            let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
            let mut play_queue = queue_lock.read().unwrap();
            (play_queue.volume / DEFAULT_VOLUME * 100.0) as usize
        };

        send(msg.channel_id, &format!("Volume: {}/100", vol), msg.tts)?;
        return Ok(());
    }

    let mut vol: usize = match args.single::<f32>() {
        Ok(vol) if vol.is_nan() => {
            send(msg.channel_id, "you're a fuck", msg.tts)?;
            return Ok(());
        },
        Ok(vol) => vol as usize,
        Err(_) => {
            send(msg.channel_id, "???????", msg.tts)?;
            return Ok(());
        },
    };

    let mut vol: f32 = (vol as f32)/100.0;  // force aliasing to reasonable values

    if vol > 3.0 {
        vol = 3.0;
    }

    if vol < 0.0 {
        vol = 0.0;
    }

    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

    {
        let mut play_queue = queue_lock.write().unwrap();
        play_queue.volume = vol * DEFAULT_VOLUME;
    }

    {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => return Ok(()),
        };

        let mut audio = current_item.audio.lock();
        audio.volume(play_queue.volume);
    };
});
