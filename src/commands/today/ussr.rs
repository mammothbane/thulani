use super::prelude::*;

pub fn ussr(date: chrono::NaiveDateTime) -> TodayIter {
    let ok = match month_day(date) {
        // red army day
        (2, 23) => true,

        // cosmonautics day
        (4, 12) => true,

        // constitution day
        (10, 7) => true,

        // november revolution day
        (11, 7) => true,

        _ => false,
    };

    if !ok {
        return Box::new(empty());
    }

    Box::new(once(by_url("https://www.youtube.com/watch?v=U06jlgpMtQs")))
}
