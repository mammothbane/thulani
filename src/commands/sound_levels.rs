use log::{
    error,
    info,
    trace,
    warn,
};
use serenity::{
    framework::standard::{
        Args,
        macros::command,
    },
    model::channel::Message,
    prelude::*,
};

use crate::{
    Result,
    CONFIG,
    audio::{PlayQueue, VoiceManager},
    util::CtxExt,
};

pub const DEFAULT_VOLUME: f32 = 0.20;
const MAX_VOLUME: f32 = 5.0;

#[command]
pub fn mute(ctx: &mut Context, _: &Message, _: Args) -> Result<()> {
    let mgr_lock = ctx.data.write().get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    manager.get_mut(CONFIG.discord.guild())
        .map(|handler| {
            if handler.self_mute {
                trace!("Already muted.")
            } else {
                handler.mute(true);
                trace!("Muted");
            }
        });

    Ok(())
}

#[command]
pub fn unmute(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let mgr_lock = ctx.data.write().get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    manager.get_mut(CONFIG.discord.guild())
        .map(|handler| {
            if !handler.self_mute {
                trace!("Already unmuted.")
            } else {
                handler.mute(false);
                trace!("Unmuted");
                let _ = ctx.send(msg.channel_id, "REEEEEEEEEEEEEE", msg.tts);
            }
        });

    Ok(())
}

#[command]
pub fn volume(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    if args.len() == 0 {
        let vol = {
            let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
            let play_queue = queue_lock.read().unwrap();
            (play_queue.volume / DEFAULT_VOLUME * 100.0) as usize
        };

        trace!("reporting volume {}", vol);

        return ctx.send(msg.channel_id, &format!("volume: {}%", vol), msg.tts);
    }

    let vol: usize = match args.single::<f32>() {
        Ok(vol) if vol.is_nan() => {
            warn!("reporting NaN volume");
            return ctx.send(msg.channel_id, "you're a fuck", msg.tts);
        },
        Ok(vol) => vol as usize,
        Err(e) => {
            error!("parsing volume arg: {}", e);
            return ctx.send(msg.channel_id, "???????", msg.tts)
        },
    };

    let mut vol: f32 = (vol as f32)/100.0;  // force aliasing to reasonable values
    let adjusted_text = if vol > MAX_VOLUME {
        format!(" ({:.0}% max)", MAX_VOLUME * 100.0)
    } else { "".to_owned() };

    vol = vol.clamp(0.0, MAX_VOLUME);

    let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();

    {
        let mut play_queue = queue_lock.write().unwrap();
        play_queue.volume = vol * DEFAULT_VOLUME;
        info!("volume updated to {}", vol);
    }

    ctx.send(msg.channel_id, format!("volume adjusted{}", adjusted_text), msg.tts)?;

    {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => return Ok(()),
        };

        let mut audio = current_item.audio.lock();
        audio.volume(play_queue.volume);
    }

    Ok(())
}
