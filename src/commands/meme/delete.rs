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
    commands::send,
    db::{
        connection,
        delete_meme,
    },
    Result,
};

pub fn delmeme(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    let title = args.single_quoted::<String>()?;

    let conn = connection()?;
    match delete_meme(&conn, &title, msg.author.id.0) {
        Ok(_) => msg.react("💀"),
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                msg.react("❓")?;
                info!("attempted to delete nonexistent meme: '{}'", title);
                send(msg.channel_id, "nice try", msg.tts)?;
                return Ok(());
            }

            Err(e)
        }
    }
}
