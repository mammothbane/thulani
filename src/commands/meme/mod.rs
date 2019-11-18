use diesel::PgConnection;
use rand::{Rng, thread_rng};
use serenity::{
    builder::CreateMessage,
    http::AttachmentType,
    model::channel::Message,
    prelude::*,
};

use crate::{
    audio::{
        PlayArgs,
        PlayQueue,
    },
    db::Meme,
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
    let should_tts = t.content.as_ref().map(|t| t.len() > 0).unwrap_or(false) &&
        thread_rng().gen::<u32>() % 25 == 0;

    debug!("sending meme (tts: {}): {:?}", should_tts, t);

    let image = t.image(conn);
    let audio = t.audio(conn);

    let create_msg = |m: &mut CreateMessage| {
        let ret = m.tts(should_tts);

        match t.content {
            Some(ref text) if text.len() > 0 => ret.content(text),
            _ => ret,
        }
    };

    match image {
        Some(image) => {
            let image = image?;
            msg.channel_id.send_files(ctx, vec!(AttachmentType::Bytes((&image.data, &image.filename))), create_msg)?;
        },

        None => match t.content {
            Some(_) => { msg.channel_id.send_message(ctx, create_msg)?; },
            None => {},
        },
    };

    // note: slight edge-case race condition here: there could have been something queued since we
    //  checked whether anything was playing. not a significant negative impact and unlikely, so i'm
    //  not worrying about it
    if let Some(audio) = audio {
        let audio = audio?;

        {
            let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
            let mut play_queue = queue_lock.write().unwrap();

            play_queue.meme_queue.push_back(PlayArgs{
                initiator: msg.author.name.clone(),
                data: ::either::Right(audio.data.clone()),
                sender_channel: msg.channel_id,
                start: None,
                end: None,
            });
        }

        msg.react(ctx, "📣")?;
    }

    Ok(())
}
