use {must_env_lookup, Result, TARGET_GUILD_ID};
use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::thread;
use std::time::Duration;

mod playback;
mod sound;

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
}

#[cfg(not(feature = "diesel"))]
fn register_db(f: StandardFramework) -> StandardFramework {
    f
}

fn send(channel: ChannelId, text: &str, tts: bool) -> Result<()> {
    channel.send_message(|m| m.content(text).tts(tts))?;
    Ok(())
}

