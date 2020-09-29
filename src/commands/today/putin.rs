use super::prelude::*;

pub fn shrek(date: chrono::NaiveDateTime) -> TodayIter {
    if month_day(date) != (10, 7) {
        return Box::new(empty());
    }

    Box::new(once(by_url("https://www.youtube.com/watch?v=tMINTOSZhDo")))
}
