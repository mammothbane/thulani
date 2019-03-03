/// This module is entirely adapted from the relevant code in Serenity.

use std::{
    process::{
        Command,
        Stdio,
    },
};

use serde_json::Value;
use serenity::{
    voice::{
        VoiceError,
    }
};

use crate::Result;

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
