use {must_env_lookup, Result, TARGET_GUILD_ID};
use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use serenity::voice::{LockedAudio, ytdl};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use typemap::Key;

pub struct VoiceManager;

const DEFAULT_VOLUME: f32 = 0.05;

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
    sender_channel: ChannelId,
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
    volume: f32,
}

impl Key for PlayQueue {
    type Value = Arc<RwLock<PlayQueue>>;
}

impl PlayQueue {
    fn new() -> Self {
        PlayQueue {
            queue: VecDeque::new(),
            playing: None,
            volume: DEFAULT_VOLUME,
        }
    }

    pub fn register(c: &mut Client) {
        let voice_manager = Arc::clone(&c.voice_manager);

        let mut data = c.data.lock();
        let queue = Arc::new(RwLock::new(PlayQueue::new()));

        data.insert::<PlayQueue>(Arc::clone(&queue));
        
        thread::spawn(move || {
            let queue_lck = Arc::clone(&queue);
            let voice_manager = voice_manager;

            loop {
                thread::sleep(Duration::from_millis(250));
                let (queue_is_empty, queue_has_playing) = {
                    let queue = queue_lck.read().unwrap();

                    let allow_continue = queue.playing.clone().map_or(false, |x| !x.audio.lock().finished);

                    if allow_continue {
                        continue;
                    }

                    (queue.queue.is_empty(), queue.playing.is_some())
                };

                if queue_is_empty {
                    if queue_has_playing {
                        let mut queue = queue_lck.write().unwrap();

                        assert!({
                            let audio_lck = queue.playing.clone().unwrap().audio;
                            let audio = audio_lck.lock();
                            audio.finished
                        });

                        queue.playing = None;

                        let mut manager = voice_manager.lock();
                        manager.leave(*TARGET_GUILD_ID);
                        debug!("disconnected due to inactivity");
                    }
                    continue;
                }

                let mut queue = queue_lck.write().unwrap();
                let item = queue.queue.pop_front().unwrap();

                trace!("checking ytdl for: {}", item.url);

                let src = match ytdl(&item.url) {
                    Ok(src) => src,
                    Err(e) => {
                        error!("bad link: {}; {:?}", &item.url, e);
                        let _ = send(item.sender_channel, &format!("what the fuck"), false);
                        continue;
                    }
                };

                trace!("got ytdl item for {}", item.url);

                let mut manager = voice_manager.lock();
                let handler = manager.join(*TARGET_GUILD_ID, must_env_lookup::<u64>("VOICE_CHANNEL"));

                match handler {
                    Some(handler) => {
                        let mut audio = handler.play_only(src);
                        {
                            audio.lock().volume(queue.volume);
                        }

                        queue.playing = Some(CurrentItem {
                            init_args: item,
                            audio,  
                        });

                        debug!("playing new song");
                    },
                    None => {
                        error!("couldn't join channel");
                        let _ = send(item.sender_channel, "something happened somewhere somehow.", false);
                    }
                }
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
        .batch_known_as(vec!["sudoku", "stop"])
        .desc("stop playing and empty the queue")
        .guild_only(true)
        .cmd(die))
    .command("meme", |c| c
        .guild_only(true)
        .help_available(false)
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
    .command("volume", |c| c
        .desc("set playback volume")
        .guild_only(true)
        .cmd(volume))
    .unrecognised_command(|ctx, msg, unrec| {
        let url = match msg.content.split_whitespace().skip(1).next() {
            Some(x) => x,
            None => {
                info!("received unrecognized command: {}", unrec);
                let _ = send(msg.channel_id, "format your commands right. fuck you.", msg.tts);
                return;
            }
        };

        let _ = _play(ctx, msg, &url);
     })
}

fn _play(ctx: &Context, msg: &Message, url: &str) -> Result<()> {
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
