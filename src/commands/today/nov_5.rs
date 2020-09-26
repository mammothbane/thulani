use super::prelude::*;

pub fn nov_5(date: chrono::NaiveDateTime) -> TodayIter {
    if (11, 5) != month_day(date) {
        return Box::new(empty());
    }

    Box::new(
        once(
            TodayArgs {
                url: "https://www.youtube.com/watch?v=LF1951pENdk",
                start: Some(Duration::seconds(25)),
                end: Some(Duration::seconds(39)),
            }
        )
    )
}
