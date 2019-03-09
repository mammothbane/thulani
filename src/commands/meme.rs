use std::{
    io::Read,
    process::{
        Command,
        Stdio,
    },
};

use diesel::{
    NotFound,
    PgConnection,
    result::Error as DieselError,
};
use failure::Error;
use rand::{Rng, thread_rng};
use serenity::{
    builder::CreateMessage,
    framework::standard::Args,
    http::AttachmentType,
    model::channel::Message,
    prelude::*,
};
use timeago::{
    Formatter,
    TimeUnit,
};
use url::Url;

use crate::{
    audio::{
        CtxExt,
        parse_times,
        PlayArgs,
        PlayQueue,
        ytdl_url,
    },
    commands::send,
    db::{
        *,
        rand_audio_meme as db_rand_audio_meme,
        rand_meme as db_rand_meme,
        rand_silent_meme as db_rand_silent_meme,
    },
    Result,
};

lazy_static! {
    static ref TIME_FORMATTER: Formatter = {
        let mut f = Formatter::new();
        f.min_unit(TimeUnit::Minutes);
        f.num_items(2);

        f
    };
}

#[inline]
pub fn meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Optional)
}

#[inline]
pub fn audio_meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Required)
}

pub fn silent_meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Prohibited)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AudioPlayback {
    Required,
    Optional,
    Prohibited,
}

fn _meme(ctx: &mut Context, msg: &Message, args: Args, audio_playback: AudioPlayback) -> Result<()> {
    if args.len() == 0 || audio_playback != AudioPlayback::Optional {
        return rand_meme(ctx, msg, AudioPlayback::Required);
    }

    let search = args.full();

    let conn = connection()?;
    let mem = match find_meme(&conn, search) {
        Ok(x) => {
            InvocationRecord::create(&conn, msg.author.id.0, msg.id.0, x.id, false)?;

            x
        },
        Err(e) => {
            return if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("requested meme not found in database");
                send(msg.channel_id, "c'mon baby, guesstimate", msg.tts)
            } else {
                send(msg.channel_id, "what in ryan's name", msg.tts)?;
                Err(e)
            };
        },
    };

    send_meme(ctx, &mem, &conn, msg)
}

pub fn wat(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let conn = connection()?;

    let record = match InvocationRecord::last(&conn) {
        Ok(x) => x,
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("found no memes in history");
                return send(msg.channel_id, "no one has ever memed before", msg.tts);
            }

            send(msg.channel_id, "BAD MEME BAD MEME", msg.tts)?;
            return Err(e);
        },
    };

    let meme = Meme::find(&conn, record.meme_id);

    match meme {
        Ok(ref meme) => {
            let metadata = Metadata::find(&conn, meme.metadata_id)?;
            let author = crate::TARGET_GUILD_ID.member(metadata.created_by as u64)?;

            send(msg.channel_id,
                 &format!("that was \"{}\" by {} ({})",
                          meme.title, author.mention(), metadata.created.date()), msg.tts)?
        },
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("last meme not found in database");
                return send(msg.channel_id, "heuueueeeeh?", msg.tts);
            }

            send(msg.channel_id, "do i look like i know what a jpeg is", msg.tts)?;
            return Err(e);
        },
    };

    meme.map(|_| {})
}

pub fn history(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    use itertools::Itertools;

    const MAX_HIST: usize = 8;
    const DEFAULT_HIST: usize = 3;

    let conn = connection()?;

    let n = args.single_quoted::<usize>().unwrap_or(DEFAULT_HIST);

    if n > MAX_HIST {
        debug!("user requested more than MAX_HIST ({}) items from history", MAX_HIST);
        send(msg.channel_id, "YER PUSHIN ME OVER THE FUCKIN LINE", true)?;
    }

    let n = n.min(MAX_HIST);

    let records = InvocationRecord::last_n(&conn, n)?;

    if records.len() == 0 {
        info!("no memes in history");
        return send(msg.channel_id, "i don't remember anything :(", msg.tts);
    }

    info!("reporting meme history (len {})", n);
    let resp = records
        .into_iter()
        .enumerate()
        .rev()
        .map(|(i, rec)| {
            use chrono;

            let dt = chrono::DateTime::from_utc(rec.time, chrono::Utc{});
            let ago = TIME_FORMATTER.convert((chrono::Utc::now() - dt).to_std().unwrap());

            let rand = if rec.random { "R, " } else { "" };
            Meme::find(&conn, rec.meme_id)
                .and_then(|meme| {
                    Metadata::find(&conn, meme.metadata_id).map(|metadata| (metadata, meme))
                })
                .map(|(metadata, meme)| {
                    let author_name = crate::TARGET_GUILD_ID.member(metadata.created_by as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    let invoker_name = crate::TARGET_GUILD_ID.member(rec.user_id as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    format!("{}. [{}{}] \"{}\" by {} ({}). invoked by {}.", i + 1, rand, ago, meme.title, author_name, metadata.created.date(), invoker_name)
                })
                .unwrap_or_else(|e| {
                    if let Some(variant) = e.downcast_ref::<DieselError>() {
                        if *variant != NotFound {
                            error!("error encountered loading meme history: {}", e);
                        }
                    }

                    let invoker_name = crate::TARGET_GUILD_ID.member(rec.user_id as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    format!("{}. [{}{}] not found. invoked by {}.", i + 1, rand, ago, invoker_name)
                })
        })
        .join("\n");

    send(msg.channel_id, &resp, false)
}

pub fn addmeme(_: &mut Context, msg: &Message, args: Args) -> Result<()> {
    let mut args = Args::new(args.rest(), &[" ".to_owned(), "\n".to_owned(), "\t".to_owned()]);

    let title = args.single_quoted::<String>()?;
    let text = args.rest().to_owned();

    let text = if text.is_empty() { None } else { Some(text) };

    let conn = connection()?;

    let image = msg.attachments.first()
        .ok_or(::failure::err_msg("no attachment"))
        .and_then(|att| {
            let data = att.download()?;
            Image::create(&conn, &att.filename, data, msg.author.id.0)
        })
        .ok();

    if image.is_none() && text.is_none() {
        warn!("tried to create non-audio meme with no image or text");
        return send(msg.channel_id, "hahAA it's empty xdddd", msg.tts);
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
        Ok(_) => msg.react("👌"),
        Err(e) => {
            if let Some(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) = e.downcast_ref::<DieselError>() {
                error!("tried to create meme that already exists");
                msg.react("❌")?;
                return send(msg.channel_id, "that meme already exists", msg.tts);
            }

            return Err(e);
        }
    }
}

pub fn addaudiomeme(_: &mut Context, msg: &Message, args: Args) -> Result<()> {
    let mut args = Args::new(args.rest(), &[" ".to_owned(), "\n".to_owned(), "\t".to_owned()]);

    let title = args.single_quoted::<String>()?;
    let audio_str = args.single_quoted::<String>()?;

    let elems = audio_str.split_whitespace().collect::<Vec<_>>();

    if elems.len() == 0 {
        send(msg.channel_id, "are you stupid", msg.tts)?;
        return Err(::failure::err_msg("no audio link was provided"))
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
        .ok_or(::failure::err_msg("no attachment"))
        .and_then(|att| {
            let data = att.download()?;
            Image::create(&conn, &att.filename, data, msg.author.id.0)
        })
        .ok();

    let mut audio_data = Vec::new();
    let bytes = audio_reader.read_to_end(&mut audio_data)?;

    if bytes == 0 {
        debug!("read 0 bytes from audio reader");
        return send(msg.channel_id, "🔇🔇🔇🔕🔕🔕🔕🔕🔇🔕🔕🔇🔕🔕📣📢📣📢📣", msg.tts);
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
        Ok(_) => msg.react("👌"),
        Err(e) => {
            if let Some(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) = e.downcast_ref::<DieselError>() {
                error!("tried to create meme that already exists");
                msg.react("❌")?;
                return send(msg.channel_id, "that meme already exists", msg.tts);
            }

            return Err(e);
        }
    }
}

pub fn delmeme(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    let title = args.single_quoted::<String>()?;

    let conn = connection()?;
    match delete_meme(&conn, &title, msg.author.id.0) {
        Ok(_) => msg.react("💀"),
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                msg.react("❓")?;
                info!("attempted to delete nonexistent meme: '{}'", title);
                send(msg.channel_id, "nice try", msg.tts)?;
                return Ok(());
            }

            Err(e)
        }
    }
}

pub fn stats(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    use db;
    use chrono;

    let conn = connection()?;
    let stats = db::stats(&conn)?;

    debug!("reporting stats");

    let s = format!(
        r#"
{} memes total
{} memes with audio ({:0.1}%)
{} memes with images ({:0.1}%)

started recording meme invocations on {} ({})
{} total meme invocations recorded
{} of which were random ({:0.1}%)
and {} were audio ({:0.1}%)"#,
        stats.memes_overall,
        stats.audio_memes,
        (stats.audio_memes as f64) / (stats.memes_overall as f64) * 100.,
        stats.image_memes,
        (stats.image_memes as f64) / (stats.memes_overall as f64) * 100.,
        stats.started_recording.date(),
        TIME_FORMATTER.convert((chrono::Utc::now() - stats.started_recording).to_std().unwrap()),
        stats.total_meme_invocations,
        stats.random_meme_invocations,
        (stats.random_meme_invocations as f64) / (stats.total_meme_invocations as f64) * 100.,
        stats.audio_meme_invocations,
        (stats.audio_meme_invocations as f64) / (stats.total_meme_invocations as f64) * 100.,
    );
    send(msg.channel_id, s, msg.tts)
}

fn rand_meme(ctx: &Context, message: &Message, audio_playback: AudioPlayback) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.users_listening()?;

    let mem = match audio_playback {
        AudioPlayback::Required => db_rand_audio_meme(&conn),
        AudioPlayback::Optional => db_rand_meme(&conn, should_audio),
        AudioPlayback::Prohibited => db_rand_silent_meme(&conn),
    };

    match mem {
        Ok(mem) => {
            InvocationRecord::create(&conn, message.author.id.0, message.id.0, mem.id, true)?;
            send_meme(ctx, &mem, &conn, message).map_err(Error::from)
        },
        Err(e) => {
            match e.downcast_ref::<DieselError>() {
                Some(NotFound) => {
                    info!("random meme not found");
                    return send(message.channel_id, "i don't know any :(", message.tts)
                },
                _ => {},
            }

            send(message.channel_id, "HELP", message.tts)?;
            return Err(e);
        },
    }
}

fn send_meme(ctx: &Context, t: &Meme, conn: &PgConnection, msg: &Message) -> Result<()> {
    debug!("sending meme: {:?}", t);

    let image = t.image(conn);
    let audio = t.audio(conn);

    let create_msg = |m: CreateMessage| {
        let ret = m
            .tts(thread_rng().gen::<u32>() % 25 == 0);

        match t.content {
            Some(ref text) => ret.content(text),
            None => ret,
        }
    };

    match image {
        Some(image) => {
            let image = image?;
            msg.channel_id.send_files(vec!(AttachmentType::Bytes((&image.data, &image.filename))), create_msg)?;
        },
        None => match t.content {
            Some(_) => { msg.channel_id.send_message(create_msg)?; },
            None => {},
        },
    };

    // note: slight edge-case race condition here: there could have been something queued since we
    //  checked whether anything was playing. not a significant negative impact and unlikely, so i'm
    //  not worrying about it
    if let Some(audio) = audio {
        let audio = audio?;
        let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.meme_queue.push_back(PlayArgs{
            initiator: msg.author.name.clone(),
            data: ::either::Right(audio.data.clone()),
            sender_channel: msg.channel_id,
            start: None,
            end: None,
        });
    }

    Ok(())
}
