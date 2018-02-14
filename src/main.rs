#[macro_use] extern crate serenity;
#[macro_use] extern crate log;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate dotenv_codegen;

extern crate dotenv;
extern crate simple_logger;
extern crate typemap;
extern crate url;

mod commands;
mod util;

mod errors {
    error_chain!();
}

use errors::*;

pub use util::*;

use std::env;
use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};

use serenity::prelude::*;
use serenity::framework::StandardFramework;
use serenity::framework::standard::help_commands;
use serenity::model::gateway::Ready;

use dotenv::dotenv;

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, _c: Context, r: Ready) {
        r.guilds.iter().find(|g| {
            g.id().0 == 0
        }).or_else(|| {
            info!("bot isn't in configured guild. let it join here: {}", OAUTH_URL);
        });
    }
}

fn run() -> Result<()> {
    let token = &env::var("DISCORD_TOKEN")?;
    let mut client = Client::new(token, Handler)?;
    let framework = StandardFramework::new()
        .configure(|c| c
            .allow_dm(false)
            .allow_whitespace(true)
            .prefixes(vec!["!thulani ", "!thulan ", "!thulando madando ", "!thulando "])
            .ignore_bots(true)
            .on_mention(false)
            .owners(HashSet::new())
            .case_insensitivity(true)
        )
        .before(|_ctx, message, cmd| {
            debug!("got command {} from user '{}' ({})", cmd, message.author.name, message.author.id);

            true
        })
        .after(|_ctx, _msg, cmd, err| {
            match err {
                Ok(()) => {},
                Err(e) => {


                }
            }
        })
        .bucket("std", 1, 10, 3)
        .customised_help(help_commands::with_embeds, |c| {
            c
        });

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
    simple_logger::init().unwrap();

    let mut backoff_count: usize = 0;

    loop {
        let start = Instant::now();

        info!("starting bot");
        match run() {
            Err(e) => {
                error!("error encountered running client: {}", e);
                e.iter().skip(1).for_each(|e| {

                });

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
        if backoff_count >= 3 {
            panic!("restarted bot too many times");
        }

        let backoff_millis = (BACKOFF_INIT*BACKOFF_FACTOR.powi(backoff_count as i32)) as u64;
        info!("bot died too quickly. backing off, retrying in {}ms.", backoff_millis);

        thread::sleep(Duration::from_millis(backoff_millis));
    }
}
