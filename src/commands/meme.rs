use std::time::Duration;

use rand::{thread_rng, distributions::{Weighted, WeightedChoice, Distribution}};
use serenity::http::AttachmentType;
use serenity::builder::CreateMessage;
use diesel::PgConnection;
use reqwest::{
    Client,
    header::{
        Headers,
        ContentLength,
        UserAgent,
        AcceptEncoding,
        Encoding,
        qitem,
    }
};

use super::*;
use super::playback::CtxExt;

use ::db::*;
use ::{Error, Result};

#[derive(Clone, Copy, Debug)]
enum MemeType {
    Text,
    Image,
    Audio,
}

static mut MEME_WEIGHTS: [Weighted<MemeType>; 3] = [
    Weighted { weight: 1, item: MemeType::Text },
    Weighted { weight: 1, item: MemeType::Image },
    Weighted { weight: 1, item: MemeType::Audio },
];

static mut TTS_WEIGHTS: [Weighted<bool>; 2] = [
    Weighted { weight: 4, item: false },
    Weighted { weight: 1, item: true }
];

command!(meme(ctx, msg, args) {
    if args.len_quoted() == 0 {
        rand_meme(ctx, msg)?;
        return Ok(());
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

            let client = Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(5))
                .build()?;

            let conn = connection()?;

            while args.len() > 0 {
                match next!().as_ref() {
                    "text" => new_meme.content = Some(args.full().to_owned()),
                    "image" => {
                        let url = args.single_quoted::<String>()?;
                        let resp = client.head(&url).send()?;

                        if !resp.status().is_success() {
                            send(msg.channel_id, "pick a better url next time thanks", msg.tts)?;
                            return Ok(());
                        }

                        let len = resp.headers().get::<ContentLength>()
                            .map(|ct_len| **ct_len)
                            .unwrap_or(0);

                        if len > 20_000_000 {
                            send(msg.channel_id, "are you trying to bankrupt my disk space", msg.tts)?;
                            return Ok(());
                        }

                        let mut resp = client.get(&url).send()?;

                        if !resp.status().is_success() {
                            send(msg.channel_id, "pick a better url next time thanks", msg.tts)?;
                            return Ok(());
                        }

                        let len = resp.headers().get::<ContentLength>()
                            .map(|ct_len| **ct_len)
                            .unwrap_or(0);

                        if len > 20_000_000 {
                            send(msg.channel_id, "are you fucking serious", msg.tts)?;
                            return Ok(());
                        }

                        if !resp.status().is_success() {
                            send(msg.channel_id, "bad link reeeeee", msg.tts)?;
                            return Ok(());
                        }

                        let mut data = Vec::with_capacity(len as usize);
                        ::std::io::copy(&mut resp, &mut data)?;

                        let image_id = Image::create(&conn, data, msg.author.id.0)?;
                        new_meme.image_id = Some(image_id);
                    },
                    "audio" | "sound" => {
                        let _url = args.single_quoted::<String>()?;
                    },
                    _ => {
                        send(msg.channel_id, "hueh?", msg.tts)?;
                        return Ok(());
                    }
                }
            }

            if new_meme.content.is_none() && new_meme.image_id.is_none() && new_meme.audio_id.is_none() {
                send(msg.channel_id, "haha it's empty lol xdddd", msg.tts)?;
                return Ok(());
            }

            new_meme.save(&conn, msg.author.id.0)?;
            send(msg.channel_id, "i hate my job", msg.tts)?;
        },
        "delete" | "remove" => {
            send(msg.channel_id, "hwaet", msg.tts)?;
        },
        search => {
            let conn = connection()?;
            let mem = match find_meme(&conn, search) {
                Ok(x) => x,
                Err(_) => {
                    send(msg.channel_id, "what in ryan's name", msg.tts)?;
                    return Ok(());
                },
            };

            send_meme(ctx, &mem, &conn, msg)?;
        }
    }
});

fn rand_meme(ctx: &Context, message: &Message) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.currently_playing() && ctx.users_listening()?;
    let weights = if should_audio {
        unsafe { &mut MEME_WEIGHTS }
    } else {
        unsafe { &mut MEME_WEIGHTS[..2] }
    };

    let dist = WeightedChoice::new(weights);

    let mut mem = match dist.sample(&mut thread_rng()) {
        MemeType::Text => rand_text(&conn),
        MemeType::Image => rand_image(&conn),
        MemeType::Audio => rand_audio(&conn),
    }.map_err(Error::from);

    mem = mem
        .and_then(|mem| {
            let mut mem = mem;

            let mut ctr = 0;
            while !should_audio && mem.audio_id.is_some() {
                mem = rand_text(&conn)?;

                ctr += 1;
                if ctr > 10 {
                    send(message.channel_id, "yer listenin to somethin else", message.tts)?;
                    return Err("looped too many times trying to find a non-audio meme".into());
                }
            }

            Ok(mem)
        });

    if let Err(e) = mem {
        send(message.channel_id, "i don't know any :(", message.tts)?;
        return Err(e);
    }

    send_meme(ctx, &mem?, &conn, message).map_err(Error::from)
}


fn send_meme(ctx: &Context, t: &Meme, conn: &PgConnection, msg: &Message) -> Result<()> {
    debug!("sending meme: {:?}", t);

    let image = t.image(conn);
    let audio = t.audio(conn);

    let dist = WeightedChoice::new(unsafe { &mut TTS_WEIGHTS });

    let create_msg = |m: CreateMessage| {
        let ret = m
            .tts(dist.sample(&mut thread_rng()));

        match t.content {
            Some(ref text) => ret.content(text),
            None => ret
        }
    };

    match image {
        Some(image) => msg.channel_id.send_files(vec!(AttachmentType::Bytes((&image?.data, &t.title))), create_msg)?,
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
