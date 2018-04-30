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
use regex::{Regex, Match};
use clap::{Arg, App, SubCommand, AppSettings};

use super::*;
use super::playback::CtxExt;

use db::*;
use failure::Error;
use Result;

lazy_static! {
    static ref COMMAND_REGEX: Regex = Regex::new(
        r"^!(?:thulani|thulando|thulando madando|thulan)\s+meme\s*(.*)"
    ).expect("unable to compile command regex");

    static ref QUOTES_REGEX: Regex = Regex::new(
        r##"^\s*(?:"([^"]*)"|([^"\s]*))\s*$"##
    ).expect("unable to compile quotes regex");

    static ref HELP_REGEX: Regex = Regex::new(
        r##"(?:^|\s)(?:--help|-h)(?:\s|$)"##
    ).expect("unable to compile help regex");
}

pub fn meme(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let arg_str = COMMAND_REGEX
        .captures(&msg.content)
        .ok_or::<Error>(::failure::err_msg("message content not recognized"))?
        .get(1)
        .ok_or::<Error>(::failure::err_msg("first capture group not found"))?
        .as_str();

    let args = QUOTES_REGEX
        .captures_iter(arg_str)
        .map(|capture| {
            capture.iter()
                .skip(1)
                .fold(None, |acc, opt| acc.or(opt))
                .map(|m: Match| m.as_str())
                .ok_or::<Error>(::failure::err_msg("couldn't extract matching group from capture"))
        })
        .collect::<Result<Vec<_>>>()?;

    let matches = match app().get_matches_from_safe_borrow(args.iter()) {
        Ok(x) => x,
        Err(_) => {
            return send(msg.channel_id, "hwaet the fuck fix your syntax", msg.tts);
        }
    };

    lazy_static! {
        static ref HELP: String = {
            let mut str = Vec::new();
            app().write_long_help(&mut str).expect("unable to write out help");
            String::from_utf8(str).expect("unable to read long help as utf8")
        };
    }

    if HELP_REGEX.is_match(arg_str) { // because clap is stupid
        return send(msg.channel_id, &format!("```{}```", &*HELP), msg.tts);
    }

    if let Some(add_matches) = matches.subcommand_matches("add") {
        lazy_static! {
            static ref ADD_HELP: String = {
                let mut str = Vec::new();
                app().write_long_help(&mut str).expect("unable to write out help");
                String::from_utf8(str).expect("unable to read long help as utf8")
            };
        }

        if HELP_REGEX.is_match(arg_str) { // because clap is stupid
            return send(msg.channel_id, &format!("```{}```", &*ADD_HELP), msg.tts);
        }

        let image = add_matches.value_of("image");
        let audio = add_matches.value_of("audio");
        let text = add_matches.value_of("text");

        let title = add_matches.value_of("title")
            .ok_or::<Error>(::failure::err_msg("somehow found no title for new meme"))?;

        if image.is_none() && audio.is_none() && text.is_none() {
            return send(msg.channel_id, "hahAA it's empty xdddd", msg.tts);
        }

        let conn = connection()?;

        lazy_static! {
            static ref CLIENT: Client = {
                let mut headers = Headers::new();
                headers.set(AcceptEncoding(vec!(qitem(Encoding::Gzip))));
                headers.set(UserAgent::new("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:59.0) Gecko/20100101 Firefox/59.0)"));
                headers.set(Accept(vec![
                    qitem(mime::IMAGE_STAR),
                    qitem("video/webm".parse().unwrap())
                ]));

                Client::builder()
                    .default_headers(headers)
                    .timeout(Duration::from_secs(5))
                    .build()
                    .expect("couldn't construct http client")
            };
        }

        let image_id = image.map(|url| load_image(&*CLIENT, &conn, url, title, msg)).transpose()?;

        if let Some(_) = audio {
            return send(msg.channel_id, "hueh?", msg.tts);
        }

        return NewMeme {
            title: title.to_owned(),
            content: text.map(|s| s.to_owned()),
            image_id,
            audio_id: None,
            metadata_id: 0,
        }.save(&conn, msg.author.id.0).map(|_| {});
    }

    if let Some(_) = matches.subcommand_matches("delete") {
        return send(msg.channel_id, "hwaet", msg.tts);
    }

    if let Some(search) = matches.value_of("SEARCH") {
        let conn = connection()?;
        let mem = match find_meme(&conn, search) {
            Ok(x) => x,
            Err(e) => {
                send(msg.channel_id, "what in ryan's name", msg.tts)?;
                return Err(e)
            },
        };

        return send_meme(ctx, &mem, &conn, msg);
    }

    rand_meme(ctx, msg)
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

fn load_image(client: &Client, conn: &PgConnection, url: &str, title: &str, msg: &Message) -> Result<i32> {
    let url = url.to_owned();
    if url.to_lowercase().trim() == "attached" {
        let res = msg.attachments.first()
            .ok_or::<Error>(::failure::err_msg("no attachments found"))
            .and_then(|att| {
                let data = att.download()?;
                let image_id = Image::create(&conn, &att.filename, data, msg.author.id.0)?;

                Ok(image_id)
            });

        if res.is_err() {
            send(msg.channel_id, "fix yer gotdang attachments", msg.tts)?;
        }

        return res;
    }

    let resp = client.head(&url).send()?;

    if !resp.status().is_success() {
        send(msg.channel_id, "pick a better url next time thanks", msg.tts)?;
        bail!("request failed");
    }

    let len = resp.headers().get::<ContentLength>()
        .map(|ct_len| **ct_len)
        .unwrap_or(0);

    let content_type_valid = resp.headers().get::<ContentType>()
        .map(|ct_type| ct_type.type_() == "image" || (ct_type.type_() == "video" && ct_type.subtype() == "webm"))
        .unwrap_or(false);

    if len > 20_000_000 || !content_type_valid {
        send(msg.channel_id, "yer pushin me over the fuckin line", msg.tts)?;
        bail!("content invalid");
    }

    let mut resp = client.get(&url).send()?;

    if !resp.status().is_success() {
        send(msg.channel_id, "bad link reeeeee", msg.tts)?;
        bail!("request failed");
    }

    let len = resp.headers().get::<ContentLength>()
        .map(|ct_len| **ct_len)
        .unwrap_or(0);

    let content_type_valid = resp.headers().get::<ContentType>()
        .map(|ct_type| ct_type.type_() == "image" || (ct_type.type_() == "video" && ct_type.subtype() == "webm"))
        .unwrap_or(false);

    if len > 20_000_000 || !content_type_valid {
        send(msg.channel_id, "are ye fuckin serious", msg.tts)?;
        bail!("content invalid");
    }

    let mut data = Vec::with_capacity(len as usize);
    ::std::io::copy(&mut resp, &mut data)?;

    let ext = resp.headers().get::<ContentType>()
        .and_then(|typ| ::mime_guess::get_extensions(typ.type_().as_str(), typ.subtype().as_str()))
        .and_then(|x| x.first())
        .unwrap_or(&"bin");

    let filename = format!("{}.{}", title, *ext);
    Image::create(conn, &filename, data, msg.author.id.0)
}

fn app<'a, 'b>() -> App<'a, 'b> {
    App::new("!thulani meme")
        .about("manipulate memes. pass no arguments to produce a randomly-selected meme.")
        .settings(&vec![AppSettings::DisableHelpSubcommand, AppSettings::DisableVersion])
        .arg(Arg::with_name("SEARCH")
            .index(1)
            .help("search for a meme by name or content (exact, case-insensitive matches only)")
        )
        .subcommand(SubCommand::with_name("add")
            .about("add a meme to the database")
            .arg(Arg::with_name("TITLE")
                .help("title for new meme")
                .required(true)
                .index(1)
            )
            .arg(Arg::with_name("image")
                .short("i")
                .long("image")
                .multiple(false)
                .help("url of image to attach (use 'attached' to use image attached to message)")
                .takes_value(true)
            )
            .arg(Arg::with_name("audio")
                .short("a")
                .long("audio")
                .multiple(false)
                .help("address of a video downloadable with youtube-dl. timestamps not yet supported.")
                .takes_value(true)
            )
            .arg(Arg::with_name("text")
                .short("t")
                .long("text")
                .multiple(false)
                .help("text to play back")
                .takes_value(true)
            )
        )
        .subcommand(SubCommand::with_name("delete")
            .about("delete a meme from the database")
            .arg(Arg::with_name("title")
                .index(1)
            )
        )
}