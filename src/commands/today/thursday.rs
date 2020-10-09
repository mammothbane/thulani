use super::prelude::*;

pub fn thursday(date: chrono::NaiveDateTime) -> TodayIter {
    if date.weekday() != chrono::Weekday::Thu {
        return Box::new(empty());
    }

    Box::new(
        once(
            by_url("https://www.youtube.com/watch?v=W6o44AfYWQE")
        )
    )
}
