/// This module is entirely adapted from the relevant code in Serenity.

use std::{
    io::{
        Read,
        Result as IoResult,
    },
    process::{
        Child,
        Command,
        Stdio,
    },
};

use chrono::Duration;
use serde_json::Value;
use serenity::{
    voice::{
        AudioSource,
        pcm,
        VoiceError,
    }
};

use crate::Result;

struct ChildContainer(Child);

impl Read for ChildContainer {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        self.0.stdout.as_mut().unwrap().read(buffer)
    }
}

impl Drop for ChildContainer {
    fn drop(&mut self) {
        if let Err(e) = self.0.kill() {
            debug!("Error awaiting child process: {:?}", e);
        }
    }
}

pub(crate) trait CodecInfo {
    fn ffmpeg_opts() -> &'static[&'static str];
}

pub(crate) struct Pcm {}
pub(crate) struct Opus {}
pub(crate) struct Mp3 {}

impl CodecInfo for Pcm {
    #[inline]
    fn ffmpeg_opts() -> &'static[&'static str] {
        lazy_static! {
            static ref OPTS: Vec<&'static str> = vec! [
                "-f", "s16le",
                "-acodec", "pcm_s16le",
            ];
        }

        &*OPTS
    }
}

impl CodecInfo for Opus {
    #[inline]
    fn ffmpeg_opts() -> &'static[&'static str] {
        lazy_static! {
            static ref OPTS: Vec<&'static str> = vec! [
//                "-f", "s16le",
                "-acodec", "libopus",
                "-sample_fmt", "s16",
                "-vbr", "off",
//                "-b:a 96k",
//                "-vn",
            ];
        }

        &*OPTS
    }
}

impl CodecInfo for Mp3 {
    #[inline]
    fn ffmpeg_opts() -> &'static[&'static str] {
        lazy_static! {
            static ref OPTS: Vec<&'static str> = vec! [
                "-f", "aac",
                "-acodec", "libfdk_aac",
                "-b:a", "96k",
                "-sample_fmt", "s16",
            ];
        }

        &*OPTS
    }
}

pub fn ytdl_url(uri: &str) -> Result<String> {
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

    match obj.remove("url") {
        Some(v) => match v {
            Value::String(uri) => Ok(uri),
            other => Err(VoiceError::YouTubeDLUrl(other).into()),
        },
        None => Err(VoiceError::YouTubeDLUrl(Value::Object(obj)).into()),
    }
}

pub(crate) fn ffmpeg_dl<T: CodecInfo>(uri: &str, start: Option<Duration>, end: Option<Duration>, size_limit: Option<usize>) -> Result<Child> {
    let start = start.unwrap_or(Duration::zero());
    let start_str = format!("{:02}:{:02}:{:02}", start.num_hours(), start.num_minutes() % 60, start.num_seconds() % 60);

    let mut opts = vec! [
        "-ac",
        "2", // force stereo -- this may cause issues
        "-ar",
        "48000",
        "-ss",
        &start_str,
    ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    let codec_opts = T::ffmpeg_opts().into_iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
    opts.extend(codec_opts);

    if let Some(limit) = size_limit {
        opts.push("-fs".to_owned());
        opts.push(format!("{}", limit));
    }

    if let Some(e) = end {
        opts.push("-to".to_owned());
        opts.push(format!("{:02}:{:02}:{:02}", e.num_hours(), e.num_minutes() % 60, e.num_seconds() % 60));
    }

    opts.push("-".to_owned());

    debug!("ffmpeg -i \"{}\" {}", uri, opts.join(" "));

    Command::new("ffmpeg")
        .arg("-i")
        .arg(uri)
        .args(opts)
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.into())
}

pub fn ytdl(uri: &str, start: Option<Duration>, end: Option<Duration>) -> Result<Box<AudioSource>> {
    let youtube_uri = ytdl_url(uri)?;
    let command = ffmpeg_dl::<Pcm>(&youtube_uri, start, end, None)?;
    Ok(pcm(true, command.stdout.unwrap()))
}
