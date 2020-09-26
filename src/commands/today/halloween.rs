use super::prelude::*;

lazy_static! {
    static ref HALLOWEEN: Vec<TodayArgs> = vec![
        TodayArgs {
            url: "https://www.youtube.com/watch?v=-1dSY6ZuXEY",
            ..Default::default()
        },
    ];
}

pub fn halloween(date: chrono::NaiveDate) -> TodayIter {
    if (10, 31) != month_day(date) {
        return Box::new(empty());
    }

    Box::new(
        HALLOWEEN.iter().cloned()
    )
}
