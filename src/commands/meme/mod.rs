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
        Audio,
        connection,
        delete_meme,
        find_meme,
        Image,
        InvocationRecord,
        Meme,
        NewMeme,
    },
    Result,
};

pub use self::{
    create::*,
    delete::*,
    history::*,
    invoke::*,
};

mod history;
mod create;
mod invoke;
mod delete;

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

        {
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

        msg.react("📣")?;
    }

    Ok(())
}
