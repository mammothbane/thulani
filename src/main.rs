#[macro_use] extern crate serenity;
#[macro_use] extern crate log;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate lazy_static;

extern crate dotenv;
extern crate fern;
extern crate typemap;
extern crate url;
extern crate chrono;

mod commands;
mod util;

use std::env;
use std::thread;
use std::time::{Duration, Instant};

use serenity::prelude::*;
use serenity::framework::StandardFramework;
use serenity::framework::standard::help_commands;
use serenity::model::gateway::Ready;
use serenity::model::id::{UserId, GuildId};

use dotenv::dotenv;

use commands::register_commands;

mod errors {
    error_chain! {
        foreign_links {
            Serenity(::serenity::Error);
            MissingVar(::std::env::VarError);
        }
    }
}

use errors::*;
pub use util::*;

lazy_static! {
    static ref TARGET_GUILD: u64 = must_env_lookup::<u64>("TARGET_GUILD");
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
    }
}

fn run() -> Result<()> {
    let token = &env::var("THULANI_TOKEN")?;
    let mut client = Client::new(token, Handler)?;

    commands::VoiceManager::register(&mut client);
    commands::PlayQueue::register(&mut client);

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
        )
        .before(|_ctx, message, cmd| {
            let result = message.guild_id().map_or(false, |x| x.0 == *TARGET_GUILD);
            debug!("got command {} from user '{}' ({}). accept: {}", cmd, message.author.name, message.author.id, result);

            result          
        })
        .after(|_ctx, _msg, _cmd, err| {
            match err {
                Ok(()) => {
                    trace!("command completed successfully");
                },
                Err(e) => {
                    error!("encountered error: {:?}", e);
                }
            }
        })
        .bucket("Standard", 1, 10, 3)
        .customised_help(help_commands::with_embeds, |c| {
            c
        });

    framework = register_commands(framework);

    client.with_framework(framework);
    client.start()?;

    Ok(())
}

fn main() {
    const BACKOFF_FACTOR: f64 = 2.0;
    const MAX_BACKOFFS: usize = 3;
    const BACKOFF_INIT: f64 = 100.0;

    const MIN_RUN_DURATION: Duration = Duration::from_secs(120);

    dotenv().ok();

    use fern::colors::{Color, ColoredLevelConfig};
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::BrightBlue)
        .trace(Color::BrightMagenta);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .level_for("thulani_rs", log::LevelFilter::Debug)
        .level_for("serenity::voice::connection", log::LevelFilter::Error)
        .chain(std::io::stdout())
        .apply()
        .expect("error initializing logging");

    let mut backoff_count: usize = 0;

    loop {
        let start = Instant::now();

        info!("starting bot");
        match run() {
            Err(e) => {
                error!("error encountered running client: {}", e);
                e.iter().skip(1).for_each(|e| {
                    error!("caused by: {}", e);
                });

                if let Some(bt) = e.backtrace() {
                    error!("backtrace: {:?}", bt);
                }

                ::std::process::exit(1);
            },
            _ => {
                warn!("somehow `run` completed without an error. should probably take a look at this.");
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