use serenity::{
    prelude::*,
    model::{
        channel::Message,
    },
    framework::standard::{
        Args,
        macros::command,
    },
};
use chrono::{Duration};
use either::Left;
use lazy_static::lazy_static;
use rand::{
    thread_rng,
    seq::SliceRandom,
};
use log::debug;

use crate::{
    Result,
    CtxExt,
    audio::{
        PlayArgs,
        PlayQueue,
    },
};

mod prelude;

mod sept_21;
mod nov_5;

mod halloween;
mod ussr;
mod france;
mod shrek;
mod putin;

mod wednesday;
mod thursday;
mod tomorrow;

mod pianoman;

pub type TodayIter = Box<dyn Iterator<Item=TodayArgs>>;

#[derive(Clone, Debug, Hash, Default)]
pub struct TodayArgs {
    pub url: &'static str,
    pub start: Option<Duration>,
    pub end: Option<Duration>,
}

impl TodayArgs {
    #[inline]
    pub fn as_play_args(&self, msg: &Message) -> PlayArgs {
        PlayArgs {
            initiator: "you have done this to yourself :^)".to_string(),
            data: Left(self.url.to_owned()),
            sender_channel: msg.channel_id,
            start: self.start,
            end: self.end,
        }
    }
}


lazy_static! {
    static ref ALL: Vec<fn(chrono::NaiveDateTime) -> TodayIter> = vec! [
        sept_21::sept_21,
        nov_5::nov_5,

        halloween::halloween,
        ussr::ussr,
        france::france,
        shrek::shrek,
        putin::putin,

        wednesday::wednesday,
        thursday::thursday,
        tomorrow::tomorrow,

        pianoman::pianoman,
    ];
}


#[command]
pub fn today(ctx: &mut Context, msg: &Message, _args: Args) -> Result<()> {
    let today = {
        #[allow(unused_mut)]
        let mut result = chrono::Local::now().naive_local();

        #[cfg(debug_assertions)] {
            let dt = _args.parse::<chrono::NaiveDateTime>()
                .or_else(|_| {
                    _args.parse::<chrono::NaiveDate>()
                        .map(|date| {
                            let time = chrono::NaiveTime::from_hms(12, 0, 0);
                            date.and_time(time)
                        })
                });

            match dt {
                Ok(dt) => {
                    log::debug!("overriding with datetime: {}", dt);
                    result = dt;
                },
                Err(e) => {
                    log::debug!("parsing datetime: {:?}", e);
                }
            };
        }

        result
    };

    let options: Vec<TodayArgs> = ALL.iter()
        .flat_map(|f| f(today))
        .collect();

    debug!("{} options for {}", options.len(), today);

    let play_args = options.choose(&mut thread_rng())
        .map(|x| x.as_play_args(msg));

    if let Some(play_args) = play_args {
        play_args.data.as_ref().left().iter().for_each(|url| {
            debug!("today selected: {}", url);
        });

        let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.general_queue.push_front(play_args);
    } else {
        ctx.send(msg.channel_id, "no", false)?;
        ctx.send(msg.channel_id, ":angry:", false)?;
    }

    Ok(())
}
