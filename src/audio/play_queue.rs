use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Cursor},
    process,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use either::{Left, Right};
use serenity::{
    prelude::*,
    voice,
};
use typemap::Key;

use crate::{
    audio::{
        CurrentItem,
        PlayArgs,
        ytdl,
    },
    commands::{
        send,
        sound_levels::DEFAULT_VOLUME,
    },
    must_env_lookup,
    TARGET_GUILD_ID,
};

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
                let mut item = queue.queue.pop_front().unwrap();

                let src = match &mut item.data {
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
                        let mut transcoder = process::Command::new("ffmpeg")
                            .args(&[
                                "-format", "opus",
                                "-i", "pipe:0",
                                "-acodec", "pcm_s16le",
                                "-f", "s16le",
                                "-"
                            ])
                            .stdin(process::Stdio::piped())
                            .stdout(process::Stdio::piped())
                            .stderr(process::Stdio::piped())
                            .spawn()
                            .expect("unable to call ffmpeg");

                        let process::Child {
                            stdin,
                            stderr,
                            stdout,
                            ..
                        } = transcoder;

                        thread::spawn(move || {
                            let stderr = BufReader::new(stderr.unwrap());

                            for line in stderr.lines() {
                                let line = line.unwrap();

                                trace!("{}", line);
                            }
                        });

                        let v = vec.clone();
                        thread::spawn(move || {
                            if let Err(e) = io::copy(&mut Cursor::new(v), &mut stdin.unwrap()) {
                                use std::io::ErrorKind;
                                if e.kind() == ErrorKind::BrokenPipe {
                                    debug!("ffmpeg closed unexpectedly");
                                } else {
                                    error!("copying audio to ffmpeg {}", e);
                                }
                            }
                        });

                        let result = voice::pcm(true, stdout.unwrap());

                        result
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

