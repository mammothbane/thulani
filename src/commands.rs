use std::sync::{Arc, Mutex as SMutex};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use serenity::prelude::*;
use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::framework::StandardFramework;
use serenity::model::id::ChannelId;
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

#[derive(Clone, Debug)]
struct PlayArgs {
    url: String,
    initiator: String,
}

#[derive(Clone)]
struct CurrentItem {
    init_args: PlayArgs,
    audio: Option<LockedAudio>,
}

#[derive(Clone)]
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
        let queue = Arc::new(SMutex::new(PlayQueue::new()));

        data.insert::<PlayQueue>(Arc::clone(&queue));
        
        thread::spawn(move || {
            let queue_lck = Arc::clone(&queue);
            let sleep = || thread::sleep(Duration::from_millis(250));

            loop {
                let mut queue = queue_lck.lock().unwrap();

                let allow_continue = queue.playing.clone().map_or(false, |ref x| match x.audio {
                    Some(ref audio) => !audio.lock().finished,
                    None => false,
                });

                if allow_continue || queue.queue.is_empty() {
                    sleep();
                    continue;
                }

                queue.advance();

                // start the music


                sleep();
            }
        });
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

fn send(channel: ChannelId, text: &str, tts: bool) -> Result<()> {
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
    .cmd("play", play)
}

command!(play(ctx, msg, args) {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            send(msg.channel_id, "BAD LINK", msg.tts)?;
            return Ok(());
        }
    };

    if !url.starts_with("http") {
        send(msg.channel_id, "bAD LiNk", msg.tts)?;
        return Ok(());
    }

    if url.contains("imgur") {
        send(msg.channel_id, "IMGUR IS BAD, YOU TRASH CAN MAN", msg.tts)?;
        return Ok(());
    }

    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    play_queue.queue.push_back(PlayArgs{
        initiator: msg.author.name.clone(),
        url,
    });
});

command!(pause(ctx, msg) {
    let mut queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    let done = || send(msg.channel_id, "r u srs", msg.tts);

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

    let done = || send(msg.channel_id, "r u srs", msg.tts);

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

command!(die(ctx, msg) {
    let data = ctx.data.lock();

    let mut mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();
    
    let mut queue_lock = data.get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.lock().unwrap();

    play_queue.playing = None;
    play_queue.queue.clear();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
    } else {
        send(msg.channel_id, "YOU die", msg.tts)?;
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
            send(msg.channel_id, &format!("Currently playing {} ({})", info.init_args.url, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            send(msg.channel_id, "Nothing is playing you fucking meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.queue.iter().for_each(|info| {
        channel.say(&format!("{} ({})", info.url, info.initiator)).unwrap(); 
    });
});

command!(meme(_ctx, msg) {
    send(msg.channel_id, "I am not yet capable of memeing", msg.tts)?;
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