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
use chrono::{Datelike, Duration};
use either::Left;
use lazy_static::lazy_static;
use rand::{
    thread_rng,
    seq::SliceRandom,
};

use crate::{
    Result,
    CtxExt,
    audio::{
        PlayArgs,
        PlayQueue,
    },
};

#[derive(Clone, Debug, Hash, Default)]
struct TodayArgs {
    url: &'static str,
    start: Option<Duration>,
    end: Option<Duration>,
}

impl TodayArgs {
    #[inline]
    fn as_play_args(&self, msg: &Message) -> PlayArgs {
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
    static ref SEPT_21_CHOICES: Vec<TodayArgs> = vec![
        TodayArgs {
            url: "https://www.youtube.com/watch?v=kPwG6L73-VU",
            ..Default::default()
        },
        TodayArgs {
            url: "https://www.youtube.com/watch?v=fPpUYXZb2AA",
            ..Default::default()
        },
        TodayArgs {
            url: "https://www.youtube.com/watch?v=CG7YHFT4hjw",
            end: Some(Duration::seconds(69)),
            ..Default::default()
        },
        TodayArgs {
            url: "https://www.youtube.com/watch?v=_hpU6UEq8hA",
            end: Some(Duration::seconds(67)),
            ..Default::default()
        },
        TodayArgs {
            url: "https://www.youtube.com/watch?v=_zzEDrYTkkg",
            end: Some(Duration::seconds(68)),
            ..Default::default()
        },
        TodayArgs {
            url: "https://www.youtube.com/watch?v=Gs069dndIYk",
            ..Default::default()
        },
    ];
}

#[command]
pub fn today(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let today = chrono::Local::today().naive_local();

    let args: Option<PlayArgs> = match (today.month(), today.day()) {
        (9, 21) => SEPT_21_CHOICES.choose(&mut thread_rng())
            .map(|choice| choice.as_play_args(msg)),
        _ => {
            let result = TodayArgs {
                url: "https://www.youtube.com/watch?v=W78AGkm_AtE",
                start: None,
                end: Some(Duration::seconds(6))
            }.as_play_args(msg);

            Some(result)
        },
    };

    if let Some(args) = args {
        let queue_lock = ctx.data.write().get::<PlayQueue>().cloned().unwrap();
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.general_queue.push_front(args);
    } else {
        ctx.send(msg.channel_id, "no", false)?;
        ctx.send(msg.channel_id, ":angry:", false)?;
    }

    Ok(())
}
