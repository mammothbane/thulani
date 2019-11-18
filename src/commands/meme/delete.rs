use diesel::{
    NotFound,
    result::Error as DieselError,
};
use serenity::{
    framework::standard::Args,
    model::channel::Message,
    prelude::*,
};

use crate::{
    db::{
        connection,
        delete_meme,
    },
    Result,
    util::CtxExt,
};

#[command]
#[aliases("delmem")]
pub fn delmeme(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    let title = args.single_quoted::<String>()?;

    let conn = connection()?;

    match delete_meme(&conn, &title, msg.author.id.0) {
        Ok(_) => msg.react(ctx, "💀"),
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                msg.react(ctx, "❓")?;
                info!("attempted to delete nonexistent meme: '{}'", title);
                ctx.send(msg.channel_id, "nice try", msg.tts)?;
                return Ok(());
            }

            Err(e)
        }
    }
}
