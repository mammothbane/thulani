use rand::{thread_rng, Rng};
use serenity::http::AttachmentType;
use serenity::builder::CreateMessage;
use serenity::framework::standard::Args;
use diesel::PgConnection;

use super::*;
use super::playback::CtxExt;

use db::*;
use failure::Error;
use Result;

pub fn meme(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    if args.len() == 0 {
        return rand_meme(ctx, msg);
    }

    let search = args.full();

    let conn = connection()?;
    let mem = match find_meme(&conn, search) {
        Ok(x) => x,
        Err(e) => {
            send(msg.channel_id, "what in ryan's name", msg.tts)?;
            return Err(e)
        },
    };

    send_meme(ctx, &mem, &conn, msg)
}

pub fn addmeme(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    let title = args.single_quoted::<String>()?;
    let text = match args.multiple_quoted::<String>() {
        Ok(text) => text.join(" "),
        Err(_) => "".to_owned(),
    };

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

    return msg.react("👌");
}

pub fn delmeme(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    send(msg.channel_id, "hwaet", msg.tts)
}

pub fn renamememe(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    send(msg.channel_id, "hwaet", msg.tts)
}

fn rand_meme(ctx: &Context, message: &Message) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.currently_playing() && ctx.users_listening()?;
    let modulus = if should_audio { 3 } else { 2 };

    let mem = match thread_rng().gen::<u32>() % modulus {
        0 => rand_text(&conn),
        1 => rand_image(&conn),
        2 => rand_audio(&conn),
        _ => unreachable!(),
    }
        .or_else(|_| rand_text(&conn))
        .and_then(|mut mem| {
            let mut ctr = 0;
            while !should_audio && mem.audio_id.is_some() {
                mem = rand_text(&conn)?;

                ctr += 1;
                if ctr > 10 {
                    send(message.channel_id, "yer listenin to somethin else", message.tts)?;
                    bail!("looped too many times trying to find a non-audio meme");
                }
            }

            Ok(mem)
        });

    match mem {
        Err(e) => {
            send(message.channel_id, "i don't know any :(", message.tts)?;
            return Err(e);
        },
        _ => {},
    }

    send_meme(ctx, &mem?, &conn, message).map_err(Error::from)
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
            None => ret
        }
    };

    match image {
        Some(image) => {
            let image = image?;
            msg.channel_id.send_files(vec!(AttachmentType::Bytes((&image.data, &image.filename))), create_msg)?
        },
        None => msg.channel_id.send_message(create_msg)?,
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
        });
    }

    Ok(())
}
