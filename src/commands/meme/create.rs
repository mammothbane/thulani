use std::{
    io::Read,
    process::{
        Command,
        Stdio,
    },
};

use diesel::result::Error as DieselError;
use log::{
    debug,
    error,
    warn,
};
use serenity::{
    framework::standard::{
        Args, CommandResult,
        Delimiter,
        macros::command,
    },
    model::channel::Message,
    prelude::*,
};
use url::Url;

use anyhow::anyhow;
use lazy_static::lazy_static;

use crate::{
    audio::{
        parse_times,
        ytdl_url,
    },
    db::{
        Audio,
        connection,
        Image,
        NewMeme,
    },
    util::CtxExt,
};

lazy_static! {
    static ref delims: Vec<Delimiter> = vec![' '.into(), '\n'.into(), '\t'.into()];
}

#[command]
pub fn addmeme(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let mut args = Args::new(args.rest(), delims.as_ref());

    let title = args.single_quoted::<String>()?;
    let text = args.rest().to_owned();

    let text = if text.is_empty() { None } else { Some(text) };

    let conn = connection()?;

    let image = msg.attachments.first()
        .ok_or(anyhow!("no attachment"))
        .and_then(|att| {
            let data = att.download()?;
            Image::create(&conn, &att.filename, data, msg.author.id.0)
        })
        .ok();

    if image.is_none() && text.is_none() {
        warn!("tried to create non-audio meme with no image or text");
        return ctx.send(msg.channel_id, "hahAA it's empty xdddd", msg.tts);
    }

    let save_result = NewMeme {
        title,
        content: text,
        image_id: image,
        audio_id: None,
        metadata_id: 0,
    }.save(&conn, msg.author.id.0).map(|_| {});

    use diesel::result::DatabaseErrorKind;
    match save_result {
        Ok(_) => msg.react(ctx, "👌"),
        Err(e) => {
            if let Some(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) = e.downcast_ref::<DieselError>() {
                error!("tried to create meme that already exists");
                msg.react(ctx, "❌")?;
                return ctx.send(msg.channel_id, "that meme already exists", msg.tts);
            }

            return Err(e);
        }
    }
}

#[command]
pub fn addaudiomeme(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let mut args = Args::new(args.rest(), delims.as_ref());

    let title = args.single_quoted::<String>()?;
    let audio_str = args.single_quoted::<String>()?;

    let elems = audio_str.split_whitespace().collect::<Vec<_>>();

    if elems.len() == 0 {
        ctx.send(msg.channel_id, "are you stupid", msg.tts)?;
        return Err(anyhow!("no audio link was provided"))
    }

    let audio_link = Url::parse(elems[0])?;
    let opts = elems[1..].join(" ");
    let (start, end) = parse_times(opts);

    let youtube_url = ytdl_url(audio_link.as_str())?;

    let duration_opts = if let Some(e) = end {
        vec! [
            "-ss".to_owned(), start.map_or_else(
                || "00:00:00".to_owned(),
                |s| format!("{:02}:{:02}:{:02}", s.num_hours(), s.num_minutes() % 60, s.num_seconds() % 60)
            ),

            "-to".to_owned(), format!("{:02}:{:02}:{:02}", e.num_hours(), e.num_minutes() % 60, e.num_seconds() % 60),
        ]
    } else {
        vec! []
    };

    let ffmpeg_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(youtube_url)
        .args(duration_opts)
        .args(&[
            "-ac", "2",
            "-ar", "48000",
            "-f", "opus",
            "-acodec", "libopus",
            "-b:a", "96k",
            "-fs", "5M",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()?;

    let mut audio_reader = ffmpeg_command.stdout.unwrap();

    let text = args.rest().to_owned();
    let text = if text.is_empty() { None } else { Some(text) };

    let conn = connection()?;

    let image = msg.attachments.first()
        .ok_or(anyhow!("no attachment"))
        .and_then(|att| {
            let data = att.download()?;
            Image::create(&conn, &att.filename, data, msg.author.id.0)
        })
        .ok();

    let mut audio_data = Vec::new();
    let bytes = audio_reader.read_to_end(&mut audio_data)?;

    if bytes == 0 {
        debug!("read 0 bytes from audio reader");
        return ctx.send(msg.channel_id, "🔇🔇🔇🔕🔕🔕🔕🔕🔇🔕🔕🔇🔕🔕📣📢📣📢📣", msg.tts);
    }

    let audio_id = Audio::create(&conn, audio_data, msg.author.id.0)?;

    let save_result = NewMeme {
        title,
        content: text,
        image_id: image,
        audio_id: Some(audio_id),
        metadata_id: 0,
    }.save(&conn, msg.author.id.0).map(|_| {});

    use diesel::result::DatabaseErrorKind;
    match save_result {
        Ok(_) => msg.react(ctx, "👌"),
        Err(e) => {
            if let Some(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) = e.downcast_ref::<DieselError>() {
                error!("tried to create meme that already exists");
                msg.react(ctx, "❌")?;
                return ctx.send(msg.channel_id, "that meme already exists", msg.tts);
            }

            return Err(e);
        }
    }
}
