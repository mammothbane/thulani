#![feature(impl_trait_in_bindings)]
#![feature(try_trait)]
#![feature(pattern)]

extern crate chrono;
#[cfg(feature = "diesel")]
#[macro_use] extern crate diesel;
extern crate dotenv;
#[macro_use] extern crate dotenv_codegen;
extern crate either;
#[macro_use] extern crate failure;
extern crate fern;
#[cfg_attr(test, macro_use)] extern crate itertools;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate rand;
extern crate regex;
extern crate serde_json;
extern crate serenity;
extern crate sha1;
extern crate time;
extern crate typemap;
extern crate url;

use std::{
    thread,
    time::{
        Duration,
        Instant
    },
};

use dotenv::dotenv;
use failure::Error;
use serenity::{
    framework::{
        standard::help_commands,
        StandardFramework,
    },
    model::{
        gateway::Ready,
        id::{GuildId, UserId},
    },
    prelude::*,
};

use self::commands::register_commands;
pub use self::util::*;

#[cfg(feature = "diesel")]
mod db;

mod commands;
mod util;
mod audio;

pub type Result<T> = ::std::result::Result<T, Error>;

lazy_static! {
    static ref TARGET_GUILD: u64 = dotenv!("TARGET_GUILD").parse().expect("unable to parse TARGET_GUILD as u64");
    static ref TARGET_GUILD_ID: GuildId = GuildId(*TARGET_GUILD);
}

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, _: Context, r: Ready) {
        let guild = r.guilds.iter()
            .find(|g| g.id().0 == *TARGET_GUILD);

        if guild.is_none() {
            info!("bot isn't in configured guild. join here: {:?}", OAUTH_URL.as_str());
        }

        #[cfg(debug_assertions)] {
            let _ = guild.map(|g| g.id().edit_nickname(Some("thulani (dev)")));
        }

        #[cfg(not(debug_assertions))] {
            let _ = guild.map(|g| g.id().edit_nickname(Some("thulani")));
        }
    }
}

fn run() -> Result<()> {
    let token = &dotenv::var("THULANI_TOKEN").map_err(|_| format_err!("missing token"))?;
    let mut client = Client::new(token, Handler)?;

    audio::VoiceManager::register(&mut client);
    audio::PlayQueue::register(&mut client);

    let owner_id = must_env_lookup::<u64>("OWNER_ID");
    let mut framework = StandardFramework::new()
        .configure(|c| c
            .allow_dm(false)
            .allow_whitespace(true)
            .prefixes(vec!["!thulani ", "!thulan ", "!thulando madando ", "!thulando "])
            .ignore_bots(true)
            .on_mention(false)
            .owners(vec![UserId(owner_id)].into_iter().collect())
            .case_insensitivity(true)
            .delimiter("\t")
        )
        .before(|_ctx, message, cmd| {
            let result = message.guild_id.map_or(false, |x| x.0 == *TARGET_GUILD);
            debug!("got command '{}' from user '{}' ({}). accept: {}", cmd, message.author.name, message.author.id, result);

            result          
        })
        .after(|_ctx, _msg, cmd, err| {
            match err {
                Ok(()) => {
                    trace!("command '{}' completed successfully", cmd);
                },
                Err(e) => {
                    error!("error encountered handling command '{}': {:?}", cmd, e);
                }
            }
        })
        .bucket("Standard", 1, 10, 3)
        .customised_help(help_commands::with_embeds, |c| {
            c
        });

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
