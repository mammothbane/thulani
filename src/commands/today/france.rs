use super::prelude::*;

pub fn france(date: chrono::NaiveDateTime) -> TodayIter {
    let ok = match month_day(date) {
        // bastille day
        (7, 14) => true,

        _ => false,
    };

    if !ok {
        return Box::new(empty());
    }

    Box::new(once(by_url("https://www.youtube.com/watch?v=VFevH5vP32s")))
}
