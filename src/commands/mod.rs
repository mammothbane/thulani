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
        .cmd(skip))
    .command("pause", |c| c
        .desc("pause playback (currently broken)")
        .guild_only(true)
        .cmd(pause))
    .command("resume", |c| c
        .desc("resume playing (currently broken)")
        .guild_only(true)
        .cmd(resume))
    .command("list", |c| c
        .known_as("queue")
        .desc("list playing and queued requests")
        .guild_only(true)
        .cmd(list))
    .command("die", |c| c
        .batch_known_as(vec!["sudoku", "stop"])
        .desc("stop playing and empty the queue")
        .guild_only(true)
        .cmd(die))
    .command("mute", |c| c
        .desc("mute thulani (playback continues)")
        .guild_only(true)
        .cmd(mute))
    .command("unmute", |c| c
        .desc("unmute thulani")
        .guild_only(true)
        .cmd(unmute))
    .command("play", |c| c
        .desc("queue a request")
        .guild_only(true)
        .cmd(play))
    .command("volume", |c| c
        .desc("set playback volume")
        .guild_only(true)
        .cmd(volume))
    .unrecognised_command(|ctx, msg, unrec| {
        let url = match msg.content.split_whitespace().skip(1).next() {
            Some(x) if x.starts_with("http") => x,
            Some(x) => {
                let _ = db_fallback(ctx, msg, x);
                return;
            },
            None => {
                info!("bad command formatting: '{}'", unrec);
                let _ = send(msg.channel_id, "format your commands right. fuck you.", msg.tts);
                return;
            }
        };

        let _ = self::playback::_play(ctx, msg, &url);
     })
}

cfg_if! {
    if #[cfg(feature = "diesel")] {
        mod meme;
        pub use self::meme::*;

        fn register_db(f: StandardFramework) -> StandardFramework {
            f
                .command("meme", |c| c
                .guild_only(true)
                .help_available(false)
                .cmd(meme))
        }
    } else {
        fn register_db(f: StandardFramework) -> StandardFramework {
            f
        }

        fn db_fallback(_: &mut Context, msg: &Message, s: &str) -> Result<()> {
            info!("received unrecognized command: {}", s);
            let _ = send(msg.channel_id, "format your commands right. fuck you.", msg.tts)?;
            Ok(())
        }
    }
}

fn send(channel: ChannelId, text: &str, tts: bool) -> Result<()> {
    channel.send_message(|m| m.content(text).tts(tts))?;
    Ok(())
}
