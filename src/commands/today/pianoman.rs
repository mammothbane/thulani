use super::prelude::*;

lazy_static! {
    static ref NINE_AM: chrono::NaiveTime = chrono::NaiveTime::from_hms(9, 0, 0);
    static ref NINE_PM: chrono::NaiveTime = chrono::NaiveTime::from_hms(21, 0, 0);

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

    let near_9am = duration_abs(*NINE_AM - dt.time()) <= Duration::minutes(5);
    let near_9pm = duration_abs(*NINE_PM - dt.time()) <= Duration::minutes(5);

    if !near_9am && !near_9pm {
        return Box::new(empty());
    }

    Box::new(PIANOMANS.iter().cloned())
}

fn duration_abs(d: Duration) -> Duration {
    if d < chrono::Duration::zero() {
        -d
    } else {
        d
    }
}
