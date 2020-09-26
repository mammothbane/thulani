#![feature(try_trait)]
#![feature(pattern)]
#![feature(concat_idents)]
#![feature(associated_type_defaults)]
#![feature(clamp)]

#![feature(box_syntax, box_patterns)]

// trash dependencies that can't be fucked to upgrade to ed. 2018
#[macro_use] extern crate diesel;
#[macro_use] extern crate pest_derive;
#[macro_use] extern crate envconfig_derive;

use std::{
    thread,
    time::{
        Duration,
        Instant,
    },
};

use log::{
    error,
    info,
};

pub use self::util::*;
pub use self::config::*;

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
mod config;
mod log_setup;
mod bot;

pub type Error = anyhow::Error;
pub type Result<T> = anyhow::Result<T>;

const BACKOFF_FACTOR: f64 = 2.0;
const MAX_BACKOFFS: usize = 3;
const BACKOFF_INIT: f64 = 100.0;

const MIN_RUN_DURATION: Duration = Duration::from_secs(120);

fn main() {
    log_setup::init().expect("initializing logging");

    let mut backoff_count: usize = 0;

    loop {
        let start = Instant::now();

        info!("starting bot");
        match bot::run() {
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
