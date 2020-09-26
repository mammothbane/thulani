use super::prelude::*;

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

pub fn sept_21(date: chrono::NaiveDateTime) -> TodayIter {
    if (9, 21) != month_day(date) {
        return Box::new(empty());
    }

    Box::new(SEPT_21_CHOICES.iter().cloned())
}
