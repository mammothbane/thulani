use diesel::{
    NotFound,
    result::Error as DieselError,
};
use itertools::Itertools;
use log::info;
use serenity::{
    framework::standard::{
        Args,
        macros::command,
    },
    model::channel::Message,
    prelude::*,
};

use crate::{
    commands::meme::send_meme,
    Result,
    db::{
        self,
        connection,
        find_meme,
        InvocationRecord,
    },
    util::CtxExt,
};

#[command]
#[aliases("mem")]
pub fn meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Optional)
}

#[command]
#[aliases("audiomeme", "audiomem")]
pub fn audio_meme(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    _meme(ctx, msg, args, AudioPlayback::Required)
}

#[command]
#[aliases("silentmeme", "silentmem")]
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

    let search = args.raw().join(" ");

    let conn = connection()?;
    let mem = match find_meme(&conn, search) {
        Ok(x) => {
            InvocationRecord::create(&conn, msg.author.id.0, msg.id.0, x.id, false)?;

            x
        },
        Err(e) => {
            return if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("requested meme not found in database");
                ctx.send(msg.channel_id, "c'mon baby, guesstimate", msg.tts)
            } else {
                ctx.send(msg.channel_id, "what in ryan's name", msg.tts)?;
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
        AudioPlayback::Required => db::rand_audio_meme(&conn),
        AudioPlayback::Optional => db::rand_meme(&conn, should_audio),
        AudioPlayback::Prohibited => db::rand_silent_meme(&conn),
    };

    match mem {
        Ok(mem) => {
            InvocationRecord::create(&conn, message.author.id.0, message.id.0, mem.id, true)?;
            send_meme(ctx, &mem, &conn, message)?;
            Ok(())
        },
        Err(e) => {
            match e.downcast_ref::<DieselError>() {
                Some(NotFound) => {
                    info!("random meme not found");
                    return ctx.send(message.channel_id, "i don't know any :(", message.tts)
                },
                _ => {},
            }

            ctx.send(message.channel_id, "HELP", message.tts)?;
            return Err(e);
        },
    }
}

#[command]
#[aliases("rarememe", "raremem")]
pub fn rare_meme(ctx: &mut Context, msg: &Message, _args: Args) -> Result<()> {
    let should_audio = ctx.users_listening()?;

    let conn = connection()?;
    let meme = db::rare_meme(&conn, should_audio);

    match meme {
        Ok(meme) => {
            InvocationRecord::create(&conn, msg.author.id.0, msg.id.0, meme.id, true)?;
            send_meme(ctx, &meme, &conn, msg)
        },
        Err(e) => {
            match e.downcast_ref::<DieselError>() {
                Some(NotFound) => {
                    info!("rare meme not found");
                    return ctx.send(msg.channel_id, "i don't know any :(", msg.tts)
                },
                _ => {},
            }

            ctx.send(msg.channel_id, "THE MEME MARKET IS IN FREEFALL", msg.tts)?;

            Err(e)
        },
    }
}
