/// This module is entirely adapted from the relevant code in Serenity.

use std::{
    io::{
        Read,
        Result as IoResult,
    },
    process::{
        Command,
        Stdio,
        Child,
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
    fn drop (&mut self) {
        if let Err(e) = self.0.kill() {
            debug!("[Voice] Error awaiting child process: {:?}", e);
        }
    }
}

pub fn ytdl_reader(uri: &str, start: Option<Duration>, end: Option<Duration>) -> Result<Box<dyn Read + Send>> {
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

    let start = start.unwrap_or(Duration::zero());
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

    Ok(Box::new(ChildContainer(command)))
}

pub fn ytdl(uri: &str, start: Option<Duration>, end: Option<Duration>) -> Result<Box<AudioSource>> {
    let command = ytdl_reader(uri, start, end)?;
    Ok(pcm(true, command))
}
