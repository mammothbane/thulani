use log::info;
use serenity::{
    framework::{
        StandardFramework,
        standard::{
            macros::group,
        },
    },
};

use crate::{
    util::CtxExt,
};

pub use self::{
    playback::*,
    sound_levels::*,
    roll::{roll, ROLL_COMMAND},
    today::{today, TODAY_COMMAND},
};
#[cfg(feature = "diesel")]
pub use self::meme::*;

pub(crate) mod playback;
pub(crate) mod sound_levels;
pub(crate) mod roll;
pub(crate) mod today;

mod help;

group!({
    name: "general",
    options: {
        only_in: "guild",
    },
    commands: [
        roll,
        today,
    ],
});

pub fn register_commands(f: StandardFramework) -> StandardFramework {
    let result = f
        .group(&self::playback::PLAYBACK_GROUP)
        .group(&GENERAL_GROUP);

    #[cfg(feature = "diesel")]
    let result = result.group(&self::meme::MEMES_GROUP);

    #[cfg(feature = "games")]
    let result = result.group(&crate::game::GAME_GROUP);

    result
        .help(&help::HELP)
        .unrecognised_command(|ctx, msg, unrec| {
            let url = match msg.content.split_whitespace().skip(1).next() {
                Some(x) if x.starts_with("http") => x,
                _ => {
                    info!("bad command formatting: '{}'", unrec);
                    let _ = ctx.send(msg.channel_id, "format your commands right. fuck you.", msg.tts);
                    return;
                }
            };

            let _ = self::playback::_play(ctx, msg, &url);
        })
}


#[cfg(feature = "diesel")]
mod meme;
