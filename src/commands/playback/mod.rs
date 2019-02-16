use either::{Left, Right};
use regex::Regex;
use time::Duration;

use serenity::{
    framework::standard::Args,
    model::{
        channel::Message,
        id::ChannelId,
    },
    prelude::*,
};

use crate::{
    commands::send,
    must_env_lookup,
    Result,
    TARGET_GUILD_ID,
};

pub use self::types::*;

mod types;

pub trait CtxExt {
    fn currently_playing(&self) -> bool;
    fn users_listening(&self) -> Result<bool>;
}

impl CtxExt for Context {
    fn currently_playing(&self) -> bool {
        let queue_lock = self.data.lock().get::<PlayQueue>().cloned().unwrap();
        let play_queue = queue_lock.read().unwrap();
        play_queue.playing.is_none()
    }

    fn users_listening(&self) -> Result<bool> {
        let channel_id = ChannelId(must_env_lookup::<u64>("VOICE_CHANNEL"));
        let channel = channel_id.to_channel()?;
        let res = channel.guild()
            .and_then(|ch| ch.read().guild())
            .map(|g| (&g.read().voice_states)
                .into_iter()
                .any(|(_, state)| state.channel_id == Some(channel_id)))
            .unwrap_or(false);

        Ok(res)
    }
}

pub fn _play(ctx: &Context, msg: &Message, url: &str) -> Result<()> {
    use url::{Url, Host};

    debug!("playing '{}'", url);
    if !url.starts_with("http") {
        send(msg.channel_id, "bAD LiNk", msg.tts)?;
        return Ok(());
    }

    let url = match Url::parse(url) {
        Err(e) => {
            send(msg.channel_id, "INVALID URL", msg.tts)?;
            return Err(e.into());
        },
        Ok(u) => u,
    };

    let host = url.host().and_then(|u| match u {
        Host::Domain(h) => Some(h.to_owned()),
        _ => None,
    });

    if host.map(|h| h.to_lowercase().contains("imgur")).unwrap_or(false) {
        send(msg.channel_id, "IMGUR IS BAD, YOU TRASH CAN MAN", msg.tts)?;
        return Ok(());
    }

    let (start, end) = parse_times(&msg.content);

    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let mut play_queue = queue_lock.write().unwrap();

    play_queue.queue.push_back(PlayArgs{
        initiator: msg.author.name.clone(),
        data: Left(url.into_string()),
        sender_channel: msg.channel_id,
        start,
        end,
    });

    Ok(())
}

lazy_static! {
    static ref START_REGEX: Regex =
        Regex::new(r"(?:start|begin(?:ning)?)\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();

    static ref DUR_REGEX: Regex =
        Regex::new(r"dur(?:ation)?\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();

    static ref END_REGEX: Regex =
        Regex::new(r"(?:end|term(?:inate|ination)?)\s*=?\s*(?:(?P<hours>\d+)h\s?)?(?:(?P<minutes>\d+)m\s?)?(?:(?P<seconds>\d+)s?)?").unwrap();
}

fn parse_times<A: AsRef<str>>(s: A) -> (Option<Duration>, Option<Duration>) {
    use regex::Match;

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
    use super::*;
    use time::Duration;

    #[test]
    fn test_start() {
        let captures = START_REGEX.captures("start 1h2m3s").unwrap();

        assert_eq!(captures.name("hours").unwrap().as_str(), "1");
        assert_eq!(captures.name("minutes").unwrap().as_str(), "2");
        assert_eq!(captures.name("seconds").unwrap().as_str(), "3");

        assert!(START_REGEX.captures("").is_none());
        assert!(START_REGEX.captures("start s").is_none());

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
        assert!(DUR_REGEX.captures("dur s").is_none());

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
        assert!(END_REGEX.captures("end s").is_none());

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

        #[inline]
        fn produce_time_strings(d: Option<Duration>, names: Vec<&'static str>) -> Box<dyn Iterator<Item=String>> {
            d
                .map(move |dur| {
                    let iter = iproduct!(format_time(&dur), names.into_iter())
                        .map(|(time, name)| format!("{} {}", name, time));
                    Box::new(iter) as Box<dyn Iterator<Item=String>>
                })
                .unwrap_or(Box::new(vec!["".to_owned()].into_iter()) as Box<dyn Iterator<Item=String>>)
        }

        #[inline]
        fn dur_strs(v: Vec<Option<Duration>>, names: Vec<&'static str>) -> Vec<String> {
            v
                .into_iter()
                .flat_map(|d| produce_time_strings(d, names.clone()))
                .collect()
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

pub fn play(ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    if args.len() == 0 {
        return _resume(ctx, msg);
    }

    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => return send(msg.channel_id, "BAD LINK", msg.tts),
    };

    _play(ctx, msg, &url)
}

pub fn pause(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

    let done = || send(msg.channel_id, "r u srs", msg.tts);
    let playing = {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => return done(),
        };

        let audio = current_item.audio.lock();
        audio.playing
    };

    if !playing {
        return done();
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().pause();
    }

    Ok(())
}

pub fn resume(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    _resume(ctx, msg)
}

fn _resume(ctx: &mut Context, msg: &Message) -> Result<()> {
    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();

    let done = || send(msg.channel_id, "r u srs", msg.tts);
    let playing = {
        let play_queue = queue_lock.read().unwrap();

        let current_item = match play_queue.playing {
            Some(ref x) => x,
            None => {
                done()?;
                return Ok(());
            },
        };

        let audio = current_item.audio.lock();
        audio.playing
    };

    if playing {
        done()?;
        return Ok(());
    }

    {
        let queue = queue_lock.write().unwrap();
        let ref audio = queue.playing.clone().unwrap().audio;
        audio.lock().play();
    }

    Ok(())
}

pub fn skip(ctx: &mut Context, _msg: &Message, _args: Args) -> Result<()> {
    let data = ctx.data.lock();

    let mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        let mut play_queue = queue_lock.write().unwrap();
        play_queue.playing = None;
    } else {
        debug!("got skip with no handler attached");
    }

    Ok(())
}

pub fn die(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let data = ctx.data.lock();

    let mgr_lock = data.get::<VoiceManager>().cloned().unwrap();
    let mut manager = mgr_lock.lock();

    let queue_lock = data.get::<PlayQueue>().cloned().unwrap();

    {
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.playing = None;
        play_queue.queue.clear();
    }

    if let Some(handler) = manager.get_mut(*TARGET_GUILD_ID) {
        handler.stop();
        handler.leave();
    } else {
        send(msg.channel_id, "YOU die", msg.tts)?;
        debug!("got die with no handler attached");
    }

    Ok(())
}

pub fn list(ctx: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
    let play_queue = queue_lock.read().unwrap();

    let channel_tmp = msg.channel().unwrap().guild().unwrap();
    let channel = channel_tmp.read();

    match play_queue.playing {
        Some(ref info) => {
            let audio = info.audio.lock();
            let status = if audio.playing { "playing" } else { "paused:" };

            let playing_info = match info.init_args.data {
                Left(ref url) => format!(" `{}`", url),
                Right(_) => "memeing".to_owned(),
            };

            send(msg.channel_id, &format!("Currently {} {} ({})", status, playing_info, info.init_args.initiator), msg.tts)?;
        },
        None => {
            debug!("`list` called with no items in queue");
            send(msg.channel_id, "Nothing is playing you meme", msg.tts)?;
            return Ok(());
        },
    }

    play_queue.queue.iter()
        .for_each(|info| {
            let playing_info = match info.data {
                Left(ref url) => format!("`{}`", url),
                Right(_) => "meme".to_owned(),
            };

            let _ = channel.say(&format!("{} ({})", playing_info, info.initiator));
        });

    Ok(())
}
