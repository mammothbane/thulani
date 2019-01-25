use std::{
    thread,
    time::Duration,
};

use serenity::{
    prelude::*,
    framework::StandardFramework,
    model::{
        channel::Message,
        id::MessageId,
    },
};

use crate::{must_env_lookup, Result, TARGET_GUILD_ID};

mod playback;
mod sound;
mod roll;

pub use self::sound::*;
pub use self::playback::*;

pub fn register_commands(f: StandardFramework) -> StandardFramework {
    let f: StandardFramework = register_db(f);
    f
        .command("skip", |c| c
            .desc("skip the rest of the current request")
            .guild_only(true)
            .exec(skip))
        .command("pause", |c| c
            .desc("pause playback (currently broken)")
            .guild_only(true)
            .exec(pause))
        .command("resume", |c| c
            .desc("resume playing (currently broken)")
            .guild_only(true)
            .exec(resume))
        .command("list", |c| c
            .known_as("queue")
            .desc("list playing and queued requests")
            .guild_only(true)
            .exec(list))
        .command("die", |c| c
            .batch_known_as(vec!["sudoku", "stop"])
            .desc("stop playing and empty the queue")
            .guild_only(true)
            .exec(die))
        .command("mute", |c| c
            .desc("mute thulani (playback continues)")
            .guild_only(true)
            .exec(mute))
        .command("unmute", |c| c
            .desc("unmute thulani")
            .guild_only(true)
            .exec(unmute))
        .command("play", |c| c
            .desc("queue a request")
            .guild_only(true)
            .exec(play))
        .command("volume", |c| c
            .desc("set playback volume")
            .guild_only(true)
            .exec(volume))
        .command("roll", |c| c
            .desc("simulate rolling dice")
            .guild_only(true)
            .exec(roll::roll))
        .unrecognised_command(|ctx, msg, unrec| {
            let url = match msg.content.split_whitespace().skip(1).next() {
                Some(x) if x.starts_with("http") => x,
                _ => {
                    info!("bad command formatting: '{}'", unrec);
                    let _ = send(msg.channel_id, "format your commands right. fuck you.", msg.tts);
                    return;
                }
            };

            let _ = self::playback::_play(ctx, msg, &url);
        })
}

#[cfg(feature = "diesel")]
mod meme;

#[cfg(feature = "diesel")]
pub use self::meme::*;

#[cfg(feature = "diesel")]
fn register_db(f: StandardFramework) -> StandardFramework {
    f
        .command("meme", |c| c
            .guild_only(true)
            .help_available(false)
            .cmd(meme))
        .command("addmeme", |c| c
            .guild_only(true)
            .desc("first argument is title, everything after is text. one attached image is included if present.")
            .cmd(addmeme)
        )
        .command("delmeme", |c| c
            .guild_only(true)
            .desc("delete a meme by name (exact match only)")
            .cmd(delmeme)
        )
        .command("renamememe", |c| c
            .guild_only(true)
            .desc("not currently working")
            .help_available(false)
            .cmd(renamememe)
        )
        .command("wat", |c| c
            .known_as("what")
            .known_as("last")
            .known_as("lastmeme")
            .guild_only(true)
            .desc("check info for last meme")
            .cmd(wat)
        )
}

#[cfg(not(feature = "diesel"))]
fn register_db(f: StandardFramework) -> StandardFramework {
    f
}

fn send<A: AsRef<str>>(channel: ChannelId, text: A, tts: bool) -> Result<()> {
    channel.send_message(|m| m.content(text.as_ref()).tts(tts))?;
    Ok(())
}

