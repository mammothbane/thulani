pub use std::iter::{
    once,
    empty,
};

pub use lazy_static::lazy_static;

pub use chrono::{
    Datelike,
    Duration,
};

pub use super::{
    TodayArgs,
    TodayIter,
};


#[inline]
pub fn month_day(date: chrono::NaiveDateTime) -> (u32, u32) {
    (date.month(), date.day())
}

pub const fn by_url(url: &'static str) -> TodayArgs {
    TodayArgs {
        url,

        start: None,
        end: None,
    }
}
