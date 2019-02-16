use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use chrono::Duration as CDuration;
use either::{Either, Left, Right};
use serenity::{
    client::bridge::voice::ClientVoiceManager,
    model::id::ChannelId,
    prelude::*,
    voice::{LockedAudio},
};
use typemap::Key;

use crate::{
    commands::{
        send,
        sound::DEFAULT_VOLUME
    },
    must_env_lookup,
    TARGET_GUILD_ID,
};

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
pub struct PlayArgs {
    pub data: Either<String, Vec<u8>>,
    pub initiator: String,
    pub sender_channel: ChannelId,
    pub start: Option<CDuration>,
    pub end: Option<CDuration>,
}

#[derive(Clone)]
pub struct CurrentItem {
    pub init_args: PlayArgs,
    pub audio: LockedAudio,
}

#[derive(Clone)]
pub struct PlayQueue {
    pub queue: VecDeque<PlayArgs>,
    pub playing: Option<CurrentItem>,
    pub volume: f32,
}

impl Key for PlayQueue {
    type Value = Arc<RwLock<PlayQueue>>;
}

impl PlayQueue {
    pub fn new() -> Self {
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
                        debug!("disconnected because playback finished");
                    }
                    continue;
                }

                let mut queue = queue_lck.write().unwrap();
                let item = queue.queue.pop_front().unwrap();

                let src = match item.data {
                    Left(ref url) => {
                        match ytdl(url, item.start, item.end) {
                            Ok(src) => src,
                            Err(e) => {
                                error!("bad link: {}; {:?}", url, e);
                                let _ = send(item.sender_channel, "what the fuck", false);
                                continue;
                            }
                        }
                    },
                    Right(ref vec) => {
                        ::serenity::voice::opus(true, ::std::io::Cursor::new(vec.clone()))
                    }
                };


                let mut manager = voice_manager.lock();
                let handler = manager.join(*TARGET_GUILD_ID, must_env_lookup::<u64>("VOICE_CHANNEL"));

                match handler {
                    Some(handler) => {
                        let audio = handler.play_only(src);
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

use std::{
    io::{
        Read,
        Result as IoResult,
        BufReader,
    },
    process::{
        Command,
        Stdio,
        Child,
    }
};

use serenity::{
    voice::{
        AudioSource,
        pcm,
    }
};
use serde_json::Value;
use crate::Result;

struct ChildContainer(Child);

impl Read for ChildContainer {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        self.0.stdout.as_mut().unwrap().read(buffer)
    }
}

impl Drop for ChildContainer {
    fn drop (&mut self) {
        if let Err(e) = self.0.kill() {
            debug!("[Voice] Error awaiting child process: {:?}", e);
        }
    }
}


// Copied from serenity
pub fn ytdl(uri: &str, start: Option<CDuration>, end: Option<CDuration>) -> Result<Box<AudioSource>> {
    let args = [
        "-f",
        "webm[abr>0]/bestaudio/best",
        "--no-playlist",
        "--print-json",
        "--skip-download",
        uri,
    ];

    let out = Command::new("youtube-dl")
        .args(&args)
        .stdin(Stdio::null())
        .output()?;

    if !out.status.success() {
        return Err(VoiceError::YouTubeDLRun(out).into());
    }

    let value = serde_json::from_reader(&out.stdout[..])?;
    let mut obj = match value {
        Value::Object(obj) => obj,
        other => return Err(VoiceError::YouTubeDLProcessing(other).into()),
    };

    let uri = match obj.remove("url") {
        Some(v) => match v {
            Value::String(uri) => uri,
            other => return Err(VoiceError::YouTubeDLUrl(other).into()),
        },
        None => return Err(VoiceError::YouTubeDLUrl(Value::Object(obj)).into()),
    };

    let start = start.unwrap_or(CDuration::zero());
    let start_str = format!("{:02}:{:02}:{:02}", start.num_hours(), start.num_minutes() % 60, start.num_seconds() % 60);

    let mut opts = vec! [
        "-f",
        "s16le",
        "-ac",
        "2", // force stereo -- this may cause issues
        "-ar",
        "48000",
        "-acodec",
        "pcm_s16le",
        "-ss",
        &start_str,
    ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    match end {
        Some(e) => {
            opts.push("-to".to_owned());
            opts.push(format!("{:02}:{:02}:{:02}", e.num_hours(), e.num_minutes() % 60, e.num_seconds() % 60));
        },
        _ => {},
    }

    opts.push("-".to_owned());

    let command = Command::new("ffmpeg")
        .arg("-i")
        .arg(uri)
        .args(opts)
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(pcm(true, ChildContainer(command)))
}
