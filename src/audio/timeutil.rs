use chrono::Duration;
use regex::{
    Match,
    Regex,
};

use lazy_static::lazy_static;

lazy_static! {
    static ref START_REGEX: Regex =
        Regex::new(r"(?:start|begin(?:ning)?)\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();

    static ref DUR_REGEX: Regex =
        Regex::new(r"dur(?:ation)?\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();

    static ref END_REGEX: Regex =
        Regex::new(r"(?:end|term(?:inate|ination)?)\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();
}

pub fn parse_times<A: AsRef<str>>(s: A) -> (Option<Duration>, Option<Duration>) {
    fn parse_match(m: Option<Match>) -> u64 {
        m.and_then(|s| s.as_str().parse::<u64>().ok()).unwrap_or(0)
    }

    fn parse_captures<B: AsRef<str>>(r: &Regex, s: B) -> Option<Duration> {
        r.captures(s.as_ref())
            .map(|capt| {
                let hours = parse_match(capt.name("hours"));
                let minutes = parse_match(capt.name("minutes"));
                let seconds = parse_match(capt.name("seconds"));

                let result = Duration::hours(hours as i64) +
                    Duration::minutes(minutes as i64) +
                    Duration::seconds(seconds as i64);

                assert!(result >= Duration::zero());

                result
            })
    }

    let start_time = parse_captures(&START_REGEX, &s);
    let dur = parse_captures(&DUR_REGEX, &s);
    let end_time = parse_captures(&END_REGEX, s)
        .or_else(|| start_time.and_then(|start| dur.map(|d| start + d)));

    (start_time, end_time)
}

#[cfg(test)]
mod test {
    use time::Duration;
    use itertools::iproduct;

    use super::*;

    #[test]
    fn test_start() {
        let captures = START_REGEX.captures("start 1h2m3s").unwrap();

        assert_eq!(captures.name("hours").unwrap().as_str(), "1");
        assert_eq!(captures.name("minutes").unwrap().as_str(), "2");
        assert_eq!(captures.name("seconds").unwrap().as_str(), "3");

        assert!(START_REGEX.captures("").is_none());

        let captures = START_REGEX.captures("start 1").unwrap();
        assert_eq!(captures.name("seconds").unwrap().as_str(), "1");
    }

    #[test]
    fn test_dur() {
        let captures = DUR_REGEX.captures("dur 1h2m3s").unwrap();

        assert_eq!(captures.name("hours").unwrap().as_str(), "1");
        assert_eq!(captures.name("minutes").unwrap().as_str(), "2");
        assert_eq!(captures.name("seconds").unwrap().as_str(), "3");

        assert!(DUR_REGEX.captures("").is_none());

        let captures = DUR_REGEX.captures("dur 1").unwrap();
        assert_eq!(captures.name("seconds").unwrap().as_str(), "1");
    }

    #[test]
    fn test_end() {
        let captures = END_REGEX.captures("end 1h2m3s").unwrap();

        assert_eq!(captures.name("hours").unwrap().as_str(), "1");
        assert_eq!(captures.name("minutes").unwrap().as_str(), "2");
        assert_eq!(captures.name("seconds").unwrap().as_str(), "3");

        assert!(END_REGEX.captures("").is_none());

        let captures = END_REGEX.captures("end 1").unwrap();
        assert_eq!(captures.name("seconds").unwrap().as_str(), "1");
    }

    #[test]
    fn test_parse_matrix() {
        fn format_time(d: &Duration) -> impl Iterator<Item=String> {
            let seconds = d.num_seconds() % 60;
            let minutes = d.num_minutes() % 60;
            let hours = d.num_hours();

            let elems = vec![true, false];

            #[inline]
            fn format_maybe_zero<S: AsRef<str>>(v: i64, unit: S, always: bool) -> String {
                if always || v != 0 {
                    format!("{}{}", v, unit.as_ref())
                } else {
                    "".to_owned()
                }
            }

            iproduct!(elems.clone(), elems.clone(), elems)
                .filter_map(move |(secs, mins, hr)| {
                    if !secs && !mins && !hr {
                        return None;
                    }

                    let hr_string = format_maybe_zero(hours, "h", hr);
                    let mn_string = format_maybe_zero(minutes, "m", mins);
                    let sec_string = format_maybe_zero(seconds, "s", secs);

                    Some(format!("{}{}{}", hr_string, mn_string, sec_string))
                })
        }

        let start_times = vec![None, Some(Duration::seconds(0)), Some(Duration::seconds(32))];
        let durs = vec![None, Some(Duration::seconds(0)), Some(Duration::seconds(123141))];
        let end_times = vec![None, Some(Duration::seconds(0)), Some(Duration::seconds(19851598))];

        let start_names = vec!["start", "begin", "beginning"];
        let dur_names = vec!["dur", "duration"];
        let end_names = vec!["end", "term", "terminate", "termination"];

        let pairs = vec! [
            (start_times, start_names),
            (durs, dur_names),
            (end_times, end_names),
        ];

        let elems = pairs.into_iter()
            .map(|(times, names)| {
                let result = times.into_iter()
                    .flat_map(move |d| {
                        let names_iter = names.clone().into_iter();

                        d.as_ref().map(move |dur| {
                            let dur = dur.clone();

                            Box::new(iproduct!(format_time(&dur), names_iter)
                                .map(move |(time, name)| Some((dur, format!("{} {}", name, time))))) as Box<dyn Iterator<Item=Option<(Duration, String)>>>
                        }).unwrap_or_else(|| Box::new(::std::iter::once(None)))
                    });

                result.collect::<Vec<Option<(Duration, String)>>>()
            })
            .collect::<Vec<Vec<Option<(Duration, String)>>>>();

        let start_iters = &elems[0];
        let dur_iters = &elems[1];
        let end_iters = &elems[2];

        iproduct!(start_iters, dur_iters, end_iters)
            .for_each(|(start, dur, end)| {
                let s = vec![start, dur, end]
                    .into_iter()
                    .filter_map(|o| {
                        o.as_ref().map(|(_, formatted)| formatted.to_owned())
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                println!("testing {}", s);

                let (parse_start, parse_end) = parse_times(s);

                match start {
                    Some((dur, _)) => assert_eq!(*dur, parse_start.unwrap()),
                    None => assert_eq!(None, parse_start),
                }

                match end {
                    Some((d, _)) => assert_eq!(*d, parse_end.unwrap()),
                    None => {
                        match dur {
                            Some((d, _)) => {
                                match start {
                                    Some((s, _)) => assert_eq!(parse_end.unwrap(), *s + *d),
                                    None => assert_eq!(None, parse_end),
                                }
                            },
                            None => assert_eq!(None, parse_end),
                        }
                    }
                }
            });
    }
}
