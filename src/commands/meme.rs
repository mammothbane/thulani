use std::time::Duration;

use rand::{thread_rng, Rng};
use serenity::http::AttachmentType;
use serenity::builder::CreateMessage;
use serenity::framework::standard::Args;
use diesel::PgConnection;
use reqwest::{
    Client,
    header::{
        Headers,
        ContentLength,
        UserAgent,
        Accept,
        AcceptEncoding,
        Encoding,
        qitem,
        ContentType,
    },
    mime
};

use super::*;
use super::playback::CtxExt;

use ::db::*;
use ::{Error, Result};

pub fn meme(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    if args.len_quoted() == 0 {
        return rand_meme(ctx, msg);
    }

    macro_rules! next { () => { args.single_quoted::<String>()?.to_lowercase() }; }

    match next!().as_ref() {
        "add" => { // e.g.: !thulani meme add title [image IMAGE] [audio|sound AUDIO] [text TEXT...]
            let mut new_meme = NewMeme {
                title: next!(),
                content: None,
                image_id: None,
                audio_id: None,
                metadata_id: 0,
            };

            let mut headers = Headers::new();
            headers.set(AcceptEncoding(vec!(qitem(Encoding::Gzip))));
            headers.set(UserAgent::new("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:59.0) Gecko/20100101 Firefox/59.0)"));
            headers.set(Accept(vec![
                qitem(mime::IMAGE_STAR),
                qitem("video/webm".parse().unwrap())
            ]));

            let client = Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(5))
                .build()?;

            let conn = connection()?;

            while args.len_quoted() > 0 {
                info!("args.len_quoted: {}; args: {:?}", args.len_quoted(), args);
                match next!().as_ref() {
                    "text" => {
                        new_meme.content = Some(args.full().to_owned());
                        break;
                    },
                    "image" => {
                        if new_meme.image_id.is_some() {
                            send(msg.channel_id, "ONLY ONE IMAGE YOU FUCK", msg.tts)?;
                            bail!("user tried to supply more than one image");
                        }

                        let mut url = args.single_quoted::<String>()?;

                        if url.to_lowercase().trim() == "attached" {
                            let res = msg.attachments.first()
                                .ok_or::<Error>(::failure::err_msg("no attachments found"))
                                .and_then(|att| {
                                    let data = att.download()?;
                                    let image_id = Image::create(&conn, &att.filename, data, msg.author.id.0)?;
                                    new_meme.image_id = Some(image_id);

                                    Ok(())
                                });

                            if res.is_err() {
                                send(msg.channel_id, "fix yer gotdang attachments", msg.tts)?;
                                return res;
                            }

                            continue;
                        }

                        let resp = client.head(&url).send()?;

                        if !resp.status().is_success() {
                            return send(msg.channel_id, "pick a better url next time thanks", msg.tts);
                        }

                        let len = resp.headers().get::<ContentLength>()
                            .map(|ct_len| **ct_len)
                            .unwrap_or(0);

                        let content_type_valid = resp.headers().get::<ContentType>()
                            .map(|ct_type| ct_type.type_() == "image" || (ct_type.type_() == "video" && ct_type.subtype() == "webm"))
                            .unwrap_or(false);

                        if len > 20_000_000 || !content_type_valid {
                            return send(msg.channel_id, "yer pushin me over the fuckin line", msg.tts);
                        }

                        let mut resp = client.get(&url).send()?;

                        if !resp.status().is_success() {
                            return send(msg.channel_id, "bad link reeeeee", msg.tts);
                        }

                        let len = resp.headers().get::<ContentLength>()
                            .map(|ct_len| **ct_len)
                            .unwrap_or(0);

                        let content_type_valid = resp.headers().get::<ContentType>()
                            .map(|ct_type| ct_type.type_() == "image" || (ct_type.type_() == "video" && ct_type.subtype() == "webm"))
                            .unwrap_or(false);

                        if len > 20_000_000 || !content_type_valid {
                            return send(msg.channel_id, "are ye fuckin serious", msg.tts);
                        }

                        let mut data = Vec::with_capacity(len as usize);
                        ::std::io::copy(&mut resp, &mut data)?;

                        let ext = resp.headers().get::<ContentType>()
                            .and_then(|typ| ::mime_guess::get_extensions(typ.type_().as_str(), typ.subtype().as_str()))
                            .and_then(|x| x.first())
                            .unwrap_or(&"bin");

                        let filename = format!("{}.{}", new_meme.title, *ext);

                        let image_id = Image::create(&conn, &filename, data, msg.author.id.0)?;
                        new_meme.image_id = Some(image_id);
                    },
                    "audio" | "sound" => {
                        let _url = args.single_quoted::<String>()?;
                    },
                    _ => {
                        return send(msg.channel_id, "hueh?", msg.tts);
                    }
                }
            }

            if new_meme.content.is_none() && new_meme.image_id.is_none() && new_meme.audio_id.is_none() {
                return send(msg.channel_id, "hahAA it's empty xdddd", msg.tts);
            }

            new_meme.save(&conn, msg.author.id.0)?;
            send(msg.channel_id, "i hate my job", msg.tts)?
        },
        "delete" | "remove" => {
            send(msg.channel_id, "hwaet", msg.tts)?
        },
        search => {
            let conn = connection()?;
            let mem = match find_meme(&conn, search) {
                Ok(x) => x,
                Err(e) => {
                    send(msg.channel_id, "what in ryan's name", msg.tts)?;
                    return Err(e)
                },
            };

            send_meme(ctx, &mem, &conn, msg)?;
        }
    }

    Ok(())
}

fn rand_meme(ctx: &Context, message: &Message) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.currently_playing() && ctx.users_listening()?;
    let modulus = if should_audio { 3 } else { 2 };

    let mut mem = match thread_rng().gen::<u32>() % modulus {
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

pub fn db_fallback(ctx: &mut Context, msg: &Message, s: &str) -> Result<()> {


    Ok(())
}
