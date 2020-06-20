#![feature(try_trait)]
#![feature(pattern)]
#![feature(concat_idents)]
#![feature(associated_type_defaults)]

#![feature(box_syntax, box_patterns)]

// trash dependencies that can't be fucked to upgrade to ed. 2018
#[macro_use] extern crate diesel;
#[macro_use] extern crate dotenv_codegen;
#[macro_use] extern crate pest_derive;

use std::{
    default::Default,
    fs::File,
    thread,
    time::{
        Duration,
        Instant,
    },
};

use chrono::Datelike;
use fnv::{FnvHashMap, FnvHashSet};
use log::{
    debug,
    error,
    info,
    trace,
    warn,
};
use serenity::{
    framework::StandardFramework,
    model::{
        gateway::Ready,
        id::{ChannelId, GuildId, MessageId, UserId},
    },
    prelude::*,
};

use anyhow::anyhow;
use dotenv::{dotenv, var as dvar};
use lazy_static::lazy_static;

use self::commands::register_commands;
pub use self::util::*;

#[cfg(feature = "diesel")]
mod db;

#[cfg(feature = "games")]
mod game;

#[cfg(not(feature = "games"))]
mod game {
    use serenity::framework::StandardFramework;

    #[inline]
    fn register(f: StandardFramework) -> StandardFramework {
        return f
    }
}

mod commands;
mod util;
mod audio;

pub type Error = anyhow::Error;

pub type Result<T> = anyhow::Result<T>;

lazy_static! {
    static ref TARGET_GUILD: u64 = dotenv!("TARGET_GUILD").parse().expect("unable to parse TARGET_GUILD as u64");
    static ref TARGET_GUILD_ID: GuildId = GuildId(*TARGET_GUILD);
    static ref VOICE_CHANNEL_ID: ChannelId = ChannelId(must_env_lookup::<u64>("VOICE_CHANNEL"));
}

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, ctx: Context, r: Ready) {
        let guild = r.guilds.iter()
            .find(|g| g.id().0 == *TARGET_GUILD);

        if guild.is_none() {
            info!("bot isn't in configured guild. join here: {:?}", OAUTH_URL.as_str());
        }

        #[cfg(debug_assertions)] {
            let _ = guild.map(|g| g.id().edit_nickname(ctx, Some("thulani (dev)")));
        }

        #[cfg(not(debug_assertions))] {
            let _ = guild.map(|g| g.id().edit_nickname(ctx, Some("thulani")));
        }
    }

    fn message_delete(&self, ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId) {
        MESSAGE_WATCH.lock()
            .remove(&deleted_message_id)
            .iter()
            .for_each(|id| {
                if let Err(e) = channel_id.delete_message(&ctx, id) {
                    error!("deleting message: {}", e);
                }
            });
    }
}

lazy_static! {
    static ref MESSAGE_WATCH: Mutex<FnvHashMap<MessageId, MessageId>> = Mutex::new(FnvHashMap::default());
    static ref PREFIXES: Vec<&'static str> = vec!["!thulani ", "!thulan ", "!thulando madando ", "!thulando "];
    static ref RESTRICTED_PREFIXES: Vec<&'static str> = vec!["!todd ", "!toddbert ", "!toddlani "];
}


fn run() -> Result<()> {
    let token = &dvar("THULANI_TOKEN").map_err(|_| anyhow!("missing token"))?;
    let mut client = Client::new(token, Handler)?;

    audio::VoiceManager::register(&mut client);
    audio::PlayQueue::register(&mut client);

    let all_prefixes = {
        let mut all_prefixes: Vec<&'static str> = vec![];
        all_prefixes.extend(PREFIXES.iter());
        all_prefixes.extend(RESTRICTED_PREFIXES.iter());
        all_prefixes
    };

    let restrict_ids = File::open("restrict.json")
        .map_err(Error::from)
        .and_then(|f| serde_json::from_reader::<_, Vec<u64>>(f).map_err(Error::from));

    if let Err(ref e) = restrict_ids {
        warn!("opening restrict file: {}", e);
    }

    let restrict_ids = restrict_ids
        .unwrap_or_default()
        .into_iter()
        .collect::<FnvHashSet<_>>();

    let owner_id = must_env_lookup::<u64>("OWNER_ID");
    let mut framework = StandardFramework::new()
        .configure(|c| c
            .allow_dm(false)
            .with_whitespace(true)
            .prefixes(all_prefixes)
            .ignore_bots(true)
            .on_mention(None)
            .owners(vec![UserId(owner_id)].into_iter().collect())
            .case_insensitivity(true)
        )
        .before(move |ctx, message, cmd| {
            debug!("got command '{}' from user '{}' ({})", cmd, message.author.name, message.author.id);
            if !message.guild_id.map_or(false, |x| x.0 == *TARGET_GUILD) {
                info!("rejecting command '{}' from user '{}': wrong guild", cmd, message.author.name);
                return false;
            }

            if message.author.id.0 == owner_id {
                return true;
            }

            let restricted_prefix = RESTRICTED_PREFIXES.iter().any(|prefix| message.content.starts_with(prefix));
            if !restricted_prefix {
                return true;
            }

            const PERMITTED_WEEKDAY: chrono::Weekday = chrono::Weekday::Tue;

            let restricted_user = restrict_ids.contains(&message.author.id.0);
            let flip_restriction_day = chrono::Local::now().weekday() == PERMITTED_WEEKDAY;

            if restricted_user == flip_restriction_day {
                return true;
            }

            let reason = if !flip_restriction_day {
                "restricted prefix".to_owned()
            } else {
                format!("it is {:?}", PERMITTED_WEEKDAY)
            };

            info!("rejecting command '{}' from user '{}': {}", cmd, message.author.name, reason);

            match ctx.send_result(message.channel_id, "no", message.tts) {
                Err(e) => error!("sending restricted prefix response: {}", e),
                Ok(msg_id) => {
                    let mut mp = MESSAGE_WATCH.lock();
                    mp.insert(message.id, msg_id);
                }
            }

            return false;
        })
        .after(|ctx, msg, cmd, err| {
            match err {
                Ok(()) => {
                    trace!("command '{}' completed successfully", cmd);
                },

                Err(e) => {
                    if let Err(e) = msg.react(&ctx, "❌") {
                        error!("reacting to failed message: {}", e);
                    }

                    if let Err(e) = ctx.send(msg.channel_id, "BANIC", msg.tts) {
                        error!("sending BANIC: {}", e);
                    }

                    error!("error encountered handling command '{}': {:?}", cmd, e);
                }
            }
        })
        .bucket("Standard", |b| b.delay(1).limit(20).time_span(60));

    framework = register_commands(framework);

    client.with_framework(framework);

    let shard_manager = client.shard_manager.clone();
    ctrlc::set_handler(move || {
        info!("shutting down");
        shard_manager.lock().shutdown_all();
    }).expect("unable to create SIGINT/SIGTERM handlers");

    client.start()?;

    Ok(())
}

fn main() {
    const BACKOFF_FACTOR: f64 = 2.0;
    const MAX_BACKOFFS: usize = 3;
    const BACKOFF_INIT: f64 = 100.0;

    const MIN_RUN_DURATION: Duration = Duration::from_secs(120);

    info!("starting");

    dotenv().ok();

    use fern::colors::{Color, ColoredLevelConfig};
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::BrightBlue)
        .trace(Color::BrightMagenta);

    fern::Dispatch::new()
        .level_for("serenity::voice::connection", log::LevelFilter::Error)
        .chain(fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{} [{}] [{}] {}",
                    chrono::Local::now().format("%_m/%_d/%y %l:%M:%S%P"),
                    colors.color(record.level()),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Warn)
            .level_for("thulani", log::LevelFilter::Debug)
            .chain(std::io::stdout())
        )
        .chain(fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] [{}] {}",
                    chrono::Local::now().format("%_m/%_d/%y %l:%M:%S%P"),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Info)
            .level_for("thulani", log::LevelFilter::Trace)
            .chain(fern::log_file("thulani.log").expect("problem creating log file"))
        )
        .apply()
        .expect("error initializing logging");

    let mut backoff_count: usize = 0;

    loop {
        let start = Instant::now();

        info!("starting bot");
        match run() {
            Err(e) => {
                error!("error encountered running client: {:?}", e);
            },
            _ => {
                // NOTE: we MUST have gotten here through SIGINT/SIGTERM handlers
                ::std::process::exit(0);
            }
        }

        if Instant::now() - start >= MIN_RUN_DURATION {
            backoff_count = 0;
            continue;
        }

        backoff_count += 1;
        if backoff_count >= MAX_BACKOFFS {
            panic!("restarted bot too many times");
        }

        let backoff_millis = (BACKOFF_INIT * BACKOFF_FACTOR.powi(backoff_count as i32)) as u64;
        info!("bot died too quickly. backing off, retrying in {}ms.", backoff_millis);

        thread::sleep(Duration::from_millis(backoff_millis));
    }
}
