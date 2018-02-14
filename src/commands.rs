use std::sync::{Arc, Mutex as SMutex};
use std::collections::VecDeque;

use serenity::prelude::*;
use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::framework::StandardFramework;
use serenity::model::channel::GuildChannel;
use serenity::voice::LockedAudio;

use typemap::Key;

use {Result, TARGET_GUILD_ID};

pub struct VoiceManager;

impl Key for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

impl VoiceManager {
    pub fn register(c: &mut Client) {
        let mut data = c.data.lock();
        data.insert::<VoiceManager>(Arc::clone(&c.voice_manager));
    }
}

struct PlayArgs {
    url: String,
    initiator: String,
}

struct CurrentItem {
    init_args: PlayArgs,
    audio: Option<LockedAudio>,
}

pub struct PlayQueue {
    queue: VecDeque<PlayArgs>,
    playing: Option<CurrentItem>,
}

impl Key for PlayQueue {
    type Value = Arc<SMutex<PlayQueue>>;
}

impl PlayQueue {
    fn new() -> Self {
        PlayQueue {
            queue: VecDeque::new(),
            playing: None,
        }
    }

    pub fn register(c: &mut Client) {
        let mut data = c.data.lock();
        data.insert::<PlayQueue>(Arc::new(SMutex::new(PlayQueue::new())));
    }

    fn advance(&mut self) {
        self.queue.pop_front().map(|info| {
            self.playing = Some(CurrentItem {
                init_args: info,
                audio: None,
            });
        });
    }
}

fn send(channel: &GuildChannel, text: &str, tts: bool) -> Result<()> {
    channel.send_message(|m| m.content(text).tts(tts))?;
    Ok(())
}

pub fn register_commands(f: StandardFramework) -> StandardFramework {
   f
    .cmd("skip", skip)
    .cmd("pause", pause)
    .cmd("resume", resume)
    .cmd("list", list)
    .cmd("die", die)
    .cmd("meme", meme)
    .cmd("mute", mute)
    .cmd("unmute", unmute)
}

command!(skip(ctx, _msg) {
    let data = ctx.data.lock();

    let mut mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();
    
    let mut queue_lock = data.get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        play_queue.advance();
    } else {
        debug!("got skip with no handler attached");
    }
});

command!(pause(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    let done = || send(&channel, "r u srs", msg.tts);

    let current_item = match play_queue.playing {
        Some(ref x) => x,
        None => {
            done()?;
            return Ok(());
        },
    };

    let locked_audio = match current_item.audio {
        Some(ref x) => x,
        None => {
            done()?;
            return Ok(());
        },
    };

    let mut audio = locked_audio.lock();

    if !audio.playing {
        done()?;
        return Ok(());
    }

    audio.pause();
});

command!(resume(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    let done = || send(&channel, "r u srs", msg.tts);

    let current_item = match play_queue.playing {
        Some(ref x) => x,
        None => {
            done()?;
            return Ok(());
        },
    };

    let locked_audio = match current_item.audio {
        Some(ref x) => x,
        None => {
            done()?;
            return Ok(());
        },
    };

    let mut audio = locked_audio.lock();

    if audio.playing {
        done()?;
        return Ok(());
    }

    audio.play();
});

command!(die(ctx, msg) {
    let data = ctx.data.lock();

    let mut mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();
    
    let mut queue_lock = data.get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    play_queue.playing = None;
    play_queue.queue.clear();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
    } else {
        send(&channel, "YOU die", msg.tts)?;
        debug!("got die with no handler attached");
    }
});

command!(list(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    match play_queue.playing {
        Some(ref info) => {
            send(&channel, &format!("Currently playing {} ({})", info.init_args.url, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            send(&channel, "Nothing is playing you fucking meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.queue.iter().for_each(|info| {
        channel.say(&format!("{} ({})", info.url, info.initiator)).unwrap(); 
    });
});

command!(meme(_ctx, _msg) {

});

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

command!(unmute(ctx, _msg) {
    let mut mgr_lock = ctx.data.lock().get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    manager.get_mut(*TARGET_GUILD_ID)
        .map(|handler| { 
            if !handler.self_mute {
                trace!("Already unmuted.")
            } else {
                handler.mute(false);
                trace!("Unmuted");
            }
        });
});