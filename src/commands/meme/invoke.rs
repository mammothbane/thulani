use diesel::result::Error as DieselError;
use failure::Error;
use serenity::{
    framework::standard::Args,
    model::channel::Message,
    prelude::*,
};

use crate::{
    audio::CtxExt,
    commands::{
        meme::send_meme,
        send,
    },
    db::{
        connection,
        find_meme,
        InvocationRecord,
        rand_audio_meme as db_rand_audio_meme,
        rand_meme as db_rand_meme,
        rand_silent_meme as db_rand_silent_meme,
    },
    Result,
};

#[inline]
pub fn meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Optional)
}

#[inline]
pub fn audio_meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Required)
}

#[inline]
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
        return rand_meme(ctx, msg, audio_playback);
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
