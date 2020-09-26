use super::prelude::*;

pub fn tomorrow(_date: chrono::NaiveDateTime) -> TodayIter {
    Box::new(
        once(
            TodayArgs {
                url: "https://www.youtube.com/watch?v=W78AGkm_AtE",
                start: None,
                end: Some(Duration::seconds(6))
            }
        )
    )
}
