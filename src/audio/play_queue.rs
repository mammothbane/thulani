use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Cursor, Read},
    process,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use either::{Left, Right};
use typemap::Key;
use log::{
    debug,
    error,
    trace,
};
use serenity::{
    CacheAndHttp,
    client::bridge::voice::ClientVoiceManager,
    prelude::*,
    voice,
};

use crate::{
    audio::{
        CurrentItem,
        PlayArgs,
        ytdl_url,
    },
    commands::{
        sound_levels::DEFAULT_VOLUME,
    },
    must_env_lookup,
    Result,
    TARGET_GUILD_ID,
};

const SECONDS_LEAD_TIME: f32 = 0.75;
const SECONDS_TRAIL_TIME: f32 = 0.1;
const SAMPLE_RATE: usize = 48000;
const CHANNELS: usize = 2;
const BYTES_PER_SAMPLE: usize = 2;
const PRE_SILENCE_BYTES: usize = (SECONDS_LEAD_TIME * (SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE) as f32) as usize;
const POST_SILENCE_BYTES: usize = (SECONDS_TRAIL_TIME * (SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE) as f32) as usize;

#[derive(Clone)]
pub struct PlayQueue {
    pub general_queue: VecDeque<PlayArgs>,
    pub meme_queue: VecDeque<PlayArgs>,
    pub playing: Option<CurrentItem>,
    pub volume: f32,
}

impl Key for PlayQueue {
    type Value = Arc<RwLock<PlayQueue>>;
}

impl PlayQueue {
    pub fn new() -> Self {
        PlayQueue {
            general_queue: VecDeque::new(),
            meme_queue: VecDeque::new(),
            playing: None,
            volume: DEFAULT_VOLUME,
        }
    }

    pub fn register(c: &mut Client) {
        let voice_manager = Arc::clone(&c.voice_manager);

        let queue = Arc::new(RwLock::new(PlayQueue::new()));

        {
            let mut data = c.data.write();
            data.insert::<PlayQueue>(Arc::clone(&queue));
        }


        let cache_http = c.cache_and_http.clone();
        thread::spawn(move || {
            loop {
                if let Err(e) = Self::update(&cache_http, &queue, &voice_manager) {
                    error!("updating playqueue: {}", e);
                }

                thread::sleep(Duration::from_millis(250));
            }
        });

    }

    fn update(cache_http: &CacheAndHttp, queue_lck: &Arc<RwLock<Self>>, voice_manager: &Arc<Mutex<ClientVoiceManager>>) -> Result<()> {
        let (queue_is_empty, queue_has_playing) = {
            let queue = queue_lck.read().unwrap();

            let allow_continue = queue.playing.clone().map_or(false, |x| !x.audio.lock().finished);

            if allow_continue {
                return Ok(());
            }

            (queue.general_queue.is_empty() && queue.meme_queue.is_empty(), queue.playing.is_some())
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

            return Ok(());
        }

        let mut queue = queue_lck.write().unwrap();

        let mut item = if !queue.meme_queue.is_empty() {
            queue.meme_queue.pop_front().unwrap()
        } else {
            queue.general_queue.pop_front().unwrap()
        };

        let mut audio_source = Cursor::new(Vec::new());
        match &mut item.data {
            Left(ref url) => {
                let youtube_url = ytdl_url(url.as_str())?;

                let duration_opts = if let Some(e) = item.end {
                    vec! [
                        "-ss".to_owned(), item.start.map_or_else(
                            || "00:00:00".to_owned(),
                            |s| format!("{:02}:{:02}:{:02}", s.num_hours(), s.num_minutes() % 60, s.num_seconds() % 60)
                        ),

                        "-to".to_owned(), format!("{:02}:{:02}:{:02}", e.num_hours(), e.num_minutes() % 60, e.num_seconds() % 60),
                    ]
                } else {
                    vec! []
                };

                let mut ffmpeg_command = process::Command::new("ffmpeg")
                    .arg("-i")
                    .arg(youtube_url)
                    .args(duration_opts)
                    .args(&[
                        "-ac", "2",
                        "-ar", "48000",
                        "-f", "s16le",
                        "-acodec", "pcm_s16le",
                        "-",
                    ])
                    .stdout(process::Stdio::piped())
                    .stderr(process::Stdio::null())
                    .stdin(process::Stdio::null())
                    .spawn()?;



                thread::spawn(move || {
                    {
                        let mut audio_reader = ffmpeg_command.stdout.as_mut();

                        if let Err(e) = io::copy(audio_reader.unwrap(), &mut audio_source) {
                            use std::io::ErrorKind;
                            if e.kind() == ErrorKind::BrokenPipe {
                                debug!("ffmpeg closed unexpectedly");
                            } else {
                                error!("copying audio from ffmpeg {}", e);
                            }
                        }
                    }

                    ffmpeg_command.wait();
                });
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

                let vec = vec.clone();
                thread::spawn(move || {
                    {
                        let process::Child {
                            stdin,
                            stderr,
                            stdout,
                            ..
                        } = &mut transcoder;

                        crossbeam::scope(|s| {
                            let writer = s.spawn(|_| {
                                if let Err(e) = io::copy(&mut Cursor::new(vec), stdin.as_mut().unwrap()) {
                                    use std::io::ErrorKind;
                                    if e.kind() == ErrorKind::BrokenPipe {
                                        debug!("ffmpeg closed unexpectedly");
                                    } else {
                                        error!("copying audio to ffmpeg {}", e);
                                    }
                                }
                            });

                            let reader = s.spawn(|_| {
                                if let Err(e) = io::copy(stdout.as_mut().unwrap(), &mut audio_source) {
                                    use std::io::ErrorKind;
                                    if e.kind() == ErrorKind::BrokenPipe {
                                        debug!("ffmpeg closed unexpectedly");
                                    } else {
                                        error!("copying audio from ffmpeg {}", e);
                                    }
                                }
                            });

                            writer.join();
                            reader.join();
                        });

                        let stderr = BufReader::new(stderr.as_mut().unwrap());

                        trace!("ffmpeg output:");
                        for line in stderr.lines() {
                            let line = line.unwrap();

                            trace!("{}", line);
                        }
                    }

                    transcoder.wait();
                });
            }
        };

        let pre_silence = vec![0u8; PRE_SILENCE_BYTES];
        let post_silence = vec![0u8; POST_SILENCE_BYTES];

        let src = voice::pcm(true, Cursor::new(pre_silence).chain(audio_source).chain(Cursor::new(post_silence)));

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
                item.sender_channel.say(&cache_http.http, "something happened somewhere somehow.")?;
            }
        }

        Ok(())
    }

}

