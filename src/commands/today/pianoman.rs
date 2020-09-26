use super::prelude::*;

lazy_static! {
    static ref TARGET_TIME: chrono::NaiveTime = chrono::NaiveTime::from_hms(21, 0, 0);

    static ref PIANOMANS: Vec<TodayArgs> = vec![
        by_url("https://www.youtube.com/watch?v=gxEPV4kolz0"),
        TodayArgs {
            url: "https://www.youtube.com/watch?v=gxEPV4kolz0",
            start: Some(Duration::seconds(30)),
            end: Some(Duration::seconds(34)),
        }
    ];
}

pub fn pianoman(dt: chrono::NaiveDateTime) -> TodayIter {
    if dt.weekday() != chrono::Weekday::Sat {
        return Box::new(empty());
    }

    let diff = {
        let result = *TARGET_TIME - dt.time();
        if result < chrono::Duration::zero() {
            -result
        } else {
            result
        }
    };

    if diff > Duration::minutes(5) {
        return Box::new(empty());
    }

    Box::new(PIANOMANS.iter().cloned())
}
