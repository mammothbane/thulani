use std::{
    io::Read,
    process::{
        Command,
        Stdio,
    },
    sync::RwLock,
};

use diesel::PgConnection;
use failure::Error;
use lazy_static::lazy_static;
use rand::{Rng, thread_rng};
use serenity::{
    builder::CreateMessage,
    framework::standard::Args,
    http::AttachmentType,
    model::channel::Message,
    prelude::*,
};
use url::Url;

use audio::ytdl_url;

use crate::{
    audio::{
        CtxExt,
        parse_times,
        PlayArgs,
        PlayQueue,
    },
    commands::send,
    db::{
        *,
        rand_meme as db_rand_meme,
    },
    Result,
};

lazy_static! {
    static ref LAST_MEME: RwLock<Option<i32>> = RwLock::new(None);
}

fn update_meme(meme: &Meme) -> Result<()> {
    let mut opt = LAST_MEME.write().map_err(|_| crate::failure::err_msg("unable to acquire lock"))?;
    *opt = Some(meme.id);

    Ok(())
}

pub fn meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    if args.len() == 0 {
        return rand_meme(ctx, msg);
    }

    let search = args.full();

    let conn = connection()?;
    let mem = match find_meme(&conn, search) {
        Ok(x) => {
            update_meme(&x)?;

            x
        },
        Err(e) => {
            use diesel::{NotFound, self};

            if let Some(NotFound) = e.downcast_ref::<diesel::result::Error>() {
                send(msg.channel_id, "c'mon baby, guesstimate", msg.tts)?;
            } else {
                send(msg.channel_id, "what in ryan's name", msg.tts)?;
            }

            return Err(e)
        },
    };

    send_meme(ctx, &mem, &conn, msg)
}

pub fn wat(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    use failure::err_msg;

    let conn = connection()?;
    let meme = LAST_MEME.read()
        .map_err(|_| err_msg("unable to acquire read lock"))
        .and_then(|id| {
            id.ok_or(err_msg("no previous meme"))
                .and_then(|id| {
                    Meme::find(&conn, id)
                })
        });

    match meme {
        Ok(ref meme) => {
            let metadata = Metadata::find(&conn, meme.metadata_id)?;
            let author = crate::TARGET_GUILD_ID.member(metadata.created_by as u64)?;

            send(msg.channel_id,
                 &format!("that was \"{}\" by {} ({})",
                          meme.title, author.mention(), metadata.created.date()), msg.tts)?
        },
        Err(_) => send(msg.channel_id, "heuueueeeeh?", msg.tts)?,
    };

    meme.map(|_| {})
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
        return send(msg.channel_id, "hahAA it's empty xdddd", msg.tts);
    }

    NewMeme {
        title,
        content: text,
        image_id: image,
        audio_id: None,
        metadata_id: 0,
    }.save(&conn, msg.author.id.0).map(|_| {})?;

    msg.react("👌")
}

pub fn addaudiomeme(_: &mut Context, msg: &Message, args: Args) -> Result<()> {
    let mut args = Args::new(args.rest(), &[" ".to_owned(), "\n".to_owned(), "\t".to_owned()]);

    let title = args.single_quoted::<String>()?;
    let audio_str = args.single_quoted::<String>()?;

    let elems = audio_str.split_whitespace().collect::<Vec<_>>();

    if elems.len() == 0 {
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

    NewMeme {
        title,
        content: text,
        image_id: image,
        audio_id: Some(audio_id),
        metadata_id: 0,
    }.save(&conn, msg.author.id.0).map(|_| {})?;

    msg.react("👌")
}

pub fn delmeme(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    let title = args.single_quoted::<String>()?;

    let conn = connection()?;
    delete_meme(&conn, &title, msg.author.id.0)?;

    msg.react("💀")
}

pub fn renamememe(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    send(msg.channel_id, "hwaet", msg.tts)
}

fn rand_meme(ctx: &Context, message: &Message) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.currently_playing() && ctx.users_listening()?;

    let mem = db_rand_meme(&conn, should_audio);

    match mem {
        Ok(mem) => {
            update_meme(&mem)?;
            send_meme(ctx, &mem, &conn, message).map_err(Error::from)
        },
        err @ Err(_) => {
            send(message.channel_id, "i don't know any :(", message.tts)?;
            err.map(|_| {})
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

        play_queue.queue.push_front(PlayArgs{
            initiator: msg.author.name.clone(),
            data: ::either::Right(audio.data.clone()),
            sender_channel: msg.channel_id,
            start: None,
            end: None,
        });
    }

    Ok(())
}
