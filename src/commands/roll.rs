use serenity::prelude::*;
use serenity::framework::standard::Args;
use serenity::model::channel::Message;
use regex::Regex;
use rand::prelude::*;

use crate::Result;

use super::send;

lazy_static! {
    static ref ROLL_REGEX: Regex = Regex::new(r"([0-9]+)?(?:d([0-9]+)(?:\s+\+\s+([0-9]+))?)")
                                        .expect("error parsing roll regex");
}

pub fn roll(_ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    let captures = match ROLL_REGEX.captures(args.full()) {
        Some(captures) => captures,
        None => return send(msg.channel_id, "conway is a goldfish", msg.tts),
    };

    let dice_count = match captures.get(1) {
        Some(x) => {
            match x.as_str().parse::<usize>() {
                Ok(x) => x,
                Err(e) => {
                    send(msg.channel_id, "conway is a goldfish", msg.tts)?;
                    return Err(e.into());
                },
            }
        },
        None => 1,
    };

    if dice_count > 1000000 {
        send(msg.channel_id, "no.", msg.tts)?;
        return Ok(());
    }

    let faces = match captures.get(2).unwrap().as_str().parse::<usize>() {
        Ok(faces) => faces,
        Err(e) => {
            send(msg.channel_id, "conway is a goldfish", msg.tts)?;
            return Err(e.into())
        },
    };

    let adjust = match captures.get(3).map(|adjust| adjust.as_str().parse::<usize>()).transpose() {
        Ok(adjust) => adjust.unwrap_or(0),
        Err(e) => {
            send(msg.channel_id, "conway is a goldfish", msg.tts)?;
            return Err(e.into())
        },
    };

    let mut rng = thread_rng();
    let total = (0..dice_count).map(|_| rng.gen_range(0, faces)).sum::<usize>() + adjust + dice_count;

    send(msg.channel_id, &format!("{}", total), msg.tts)
}
