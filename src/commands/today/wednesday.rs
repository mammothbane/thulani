use super::prelude::*;

pub fn wednesday(date: chrono::NaiveDate) -> TodayIter {
    if date.weekday() != chrono::Weekday::Wed {
        return Box::new(empty());
    }

    Box::new(
        once(
            by_url("https://www.youtube.com/watch?v=du-TY1GUFGk")
        )
    )
}
