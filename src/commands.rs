use std::sync::{Arc, Mutex as SMutex};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use serenity::prelude::*;
use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::framework::StandardFramework;
use serenity::model::id::ChannelId;
use serenity::voice::{LockedAudio, ytdl};

use typemap::Key;

use {Result, TARGET_GUILD_ID, must_env_lookup};

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
    audio: LockedAudio,
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
        let voice_manager = Arc::clone(&c.voice_manager);

        let mut data = c.data.lock();
        let queue = Arc::new(SMutex::new(PlayQueue::new()));

        data.insert::<PlayQueue>(Arc::clone(&queue));
        
        thread::spawn(move || {
            let queue_lck = Arc::clone(&queue);
            let sleep = || thread::sleep(Duration::from_millis(250));
            let voice_manager = voice_manager;

            let channel = ChannelId(must_env_lookup("DEFAULT_CHANNEL"));

            loop {
                let mut queue = queue_lck.lock().unwrap();

                let allow_continue = queue.playing.clone().map_or(false, |x| !x.audio.lock().finished);

                if allow_continue {
                    sleep();
                    continue;
                }

                if queue.queue.is_empty() {
                    if queue.playing.is_some() { // must be finished
                        assert!({
                            let audio_lck = queue.playing.clone().unwrap().audio;
                            let audio = audio_lck.lock();
                            audio.finished
                        });

                        let mut manager = voice_manager.lock();
                        manager.leave(*TARGET_GUILD_ID);
                    }

                    sleep();
                    continue;
                }

                let item = queue.queue.pop_front().unwrap();
                let src = match ytdl(&item.url) {
                    Ok(src) => src,
                    Err(e) => {
                        error!("bad link: {}; {}", &item.url, e);
                        let _ = send(channel, &format!("what the fuck"), false);
                        sleep();
                        continue;
                    }
                };

                let mut manager = voice_manager.lock();
                let handler = manager.join(*TARGET_GUILD_ID, must_env_lookup::<u64>("VOICE_CHANNEL"));

                match handler {
                    Some(handler) => {
                        let audio = handler.play_only(src);
                        queue.playing = Some(CurrentItem {
                            init_args: item,
                            audio,  
                        });
                    },
                    None => {
                        error!("couldn't join channel");
                        let _ = send(channel, "something happened somewhere somehow.", false);
                    }
                }

                sleep();
            }
        });
    }
}

fn send(channel: ChannelId, text: &str, tts: bool) -> Result<()> {
    channel.send_message(|m| m.content(text).tts(tts))?;
    Ok(())
}

pub fn register_commands(f: StandardFramework) -> StandardFramework {
   f
    .command("skip", |c| c
        .desc("skip the rest of the current request")
        .guild_only(true)
        .cmd(skip))
    .command("pause", |c| c
        .desc("pause playback (currently broken)")
        .guild_only(true)
        .cmd(pause))
    .command("resume", |c| c
        .desc("resume playing (currently broken)")
        .guild_only(true)
        .cmd(resume))
    .command("list", |c| c
        .known_as("queue")
        .desc("list playing and queued requests")
        .guild_only(true)
        .cmd(list))
    .command("die", |c| c
        .known_as("sudoku")
        .desc("stop playing and empty the queue")
        .guild_only(true)
        .cmd(die))
    .command("meme", |c| c
        .guild_only(true)
        .cmd(meme))
    .command("mute", |c| c
        .desc("mute thulani (playback continues)")
        .guild_only(true)
        .cmd(mute))
    .command("unmute", |c| c
        .desc("unmute thulani")
        .guild_only(true)
        .cmd(unmute))
    .command("play", |c| c
        .desc("queue a request")
        .guild_only(true)
        .cmd(play))
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

    let mut audio = current_item.audio.lock();

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

    let mut audio = current_item.audio.lock();

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
    let mut play_queue = queue_lock.lock().unwrap();

    play_queue.playing = None;
    play_queue.queue.clear();

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
    let mut play_queue = queue_lock.lock().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    match play_queue.playing {
        Some(ref info) => {
            send(msg.channel_id, &format!("Currently playing `{}` ({})", info.init_args.url, info.init_args.initiator), msg.tts)?;
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