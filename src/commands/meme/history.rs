use diesel::{
    NotFound,
    result::Error as DieselError,
};
use serenity::{
    framework::standard::Args,
    model::channel::Message,
    prelude::*,
};
use timeago::{
    Formatter,
    TimeUnit,
};

use crate::{
    commands::send,
    db::{
        connection,
        InvocationRecord,
        Meme,
        Metadata,
    },
    must_env_lookup,
    Result,
};

lazy_static! {
    static ref TIME_FORMATTER: Formatter = {
        let mut f = Formatter::new();
        f.min_unit(TimeUnit::Minutes);
        f.num_items(2);

        f
    };
}

static CLEAN_DATE_FORMAT: &'static str = "%b %-e %Y";

pub fn wat(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    let conn = connection()?;

    let record = match InvocationRecord::last(&conn) {
        Ok(x) => x,
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("found no memes in history");
                return send(msg.channel_id, "no one has ever memed before", msg.tts);
            }

            send(msg.channel_id, "BAD MEME BAD MEME", msg.tts)?;
            return Err(e);
        },
    };

    let meme = Meme::find(&conn, record.meme_id);

    match meme {
        Ok(ref meme) => {
            let metadata = Metadata::find(&conn, meme.metadata_id)?;
            let author = crate::TARGET_GUILD_ID.member(metadata.created_by as u64)?;

            send(msg.channel_id,
                 &format!("that was \"{}\" by {} ({})",
                          meme.title, author.mention(), metadata.created.date().format(CLEAN_DATE_FORMAT)), msg.tts)?
        },
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("last meme not found in database");
                return send(msg.channel_id, "heuueueeeeh?", msg.tts);
            }

            send(msg.channel_id, "do i look like i know what a jpeg is", msg.tts)?;
            return Err(e);
        },
    };

    meme.map(|_| {})
}

pub fn history(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    use itertools::Itertools;

    lazy_static! {
        static ref MAX_HIST: usize = must_env_lookup("MAX_HIST");
        static ref DEFAULT_HIST: usize = must_env_lookup("DEFAULT_HIST");
    }

    let conn = connection()?;

    let n = args.single_quoted::<usize>().unwrap_or(*DEFAULT_HIST);

    if n > *MAX_HIST {
        debug!("user requested more than MAX_HIST ({}) items from history", *MAX_HIST);
        send(msg.channel_id, "YER PUSHIN ME OVER THE FUCKIN LINE", true)?;
    }

    let n = n.min(*MAX_HIST);

    let records = InvocationRecord::last_n(&conn, n)?;

    if records.len() == 0 {
        info!("no memes in history");
        return send(msg.channel_id, "i don't remember anything :(", msg.tts);
    }

    info!("reporting meme history (len {})", n);
    let resp = records
        .into_iter()
        .enumerate()
        .rev()
        .map(|(i, rec)| {
            use chrono;

            let dt = chrono::DateTime::from_utc(rec.time, chrono::Utc{});
            let ago = TIME_FORMATTER.convert((chrono::Utc::now() - dt).to_std().unwrap());

            let rand = if rec.random { "R, " } else { "" };
            Meme::find(&conn, rec.meme_id)
                .and_then(|meme| {
                    Metadata::find(&conn, meme.metadata_id).map(|metadata| (metadata, meme))
                })
                .map(|(metadata, meme)| {
                    let author_name = crate::TARGET_GUILD_ID.member(metadata.created_by as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    let invoker_name = crate::TARGET_GUILD_ID.member(rec.user_id as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    format!("{}. [{}{}] \"{}\" by {} ({}). invoked by {}.", i + 1, rand, ago, meme.title, author_name, metadata.created.date().format(CLEAN_DATE_FORMAT), invoker_name)
                })
                .unwrap_or_else(|e| {
                    if let Some(variant) = e.downcast_ref::<DieselError>() {
                        if *variant != NotFound {
                            error!("error encountered loading meme history: {}", e);
                        }
                    }

                    let invoker_name = crate::TARGET_GUILD_ID.member(rec.user_id as u64).map(|m| m.display_name().into_owned()).unwrap_or("???".to_owned());
                    format!("{}. [{}{}] not found. invoked by {}.", i + 1, rand, ago, invoker_name)
                })
        })
        .join("\n");

    send(msg.channel_id, &resp, false)
}

pub fn stats(_: &mut Context, msg: &Message, _: Args) -> Result<()> {
    use db;
    use chrono;
    use serenity::model::{
        id::UserId,
        user::User,
    };
    use crate::TARGET_GUILD_ID;

    let conn = connection()?;
    let stats = db::stats(&conn)?;

    debug!("reporting stats");

    let rand_user: User = UserId(stats.most_random_meme_user).to_user()?;
    let direct_user: User = UserId(stats.most_directly_named_meme_user).to_user()?;

    let rand_user = rand_user.nick_in(*TARGET_GUILD_ID).unwrap_or(rand_user.name);
    let direct_user = direct_user.nick_in(*TARGET_GUILD_ID).unwrap_or(direct_user.name);

    let s = format!(
        r#"
**{}** memes stored
**{}** memes with audio ({:0.1}%)
**{}** memes with images ({:0.1}%)

started recording meme invocations on *{}* ({})
**{}** total meme invocations recorded
**{}** of which were random ({:0.1}%)
and **{}** were audio ({:0.1}%)

the most active day was *{}* with **{}** memes
and the loudest day was *{}* with **{}** audio memes

**{}** has invoked the most random memes ({})
**{}** has invoked the most memes by name ({})

*{}* was the meme specifically requested the most ({})
*{}* was the meme randomly invoked the most ({})
and *{}* was the most-memed overall ({})"#,
        stats.memes_overall,
        stats.audio_memes,
        (stats.audio_memes as f64) / (stats.memes_overall as f64) * 100.,
        stats.image_memes,
        (stats.image_memes as f64) / (stats.memes_overall as f64) * 100.,
        stats.started_recording.date().format(CLEAN_DATE_FORMAT),
        TIME_FORMATTER.convert((chrono::Utc::now() - stats.started_recording).to_std().unwrap()),
        stats.total_meme_invocations,
        stats.random_meme_invocations,
        (stats.random_meme_invocations as f64) / (stats.total_meme_invocations as f64) * 100.,
        stats.audio_meme_invocations,
        (stats.audio_meme_invocations as f64) / (stats.total_meme_invocations as f64) * 100.,
        stats.most_active_day.format(CLEAN_DATE_FORMAT), stats.most_active_day_count,
        stats.most_audio_active_day.format(CLEAN_DATE_FORMAT), stats.most_audio_active_count,
        rand_user, stats.most_random_meme_user_count,
        direct_user, stats.most_directly_named_meme_count,
        stats.most_popular_named_meme, stats.most_popular_named_meme_count,
        stats.most_popular_random_meme, stats.most_popular_random_meme_count,
        stats.most_popular_meme_overall, stats.most_popular_meme_overall_count,
    );
    send(msg.channel_id, s, msg.tts)
}

pub fn memers(_: &mut Context, msg: &Message, _args: Args) -> Result<()> {
    use db;
    use itertools::Itertools;
    use serenity::model::{
        id::UserId,
    };
    use crate::TARGET_GUILD_ID;

    let s = db::memers()?
        .into_iter()
        .map(|info| {
            let user = UserId(info.user_id).to_user()?;
            let username = user.nick_in(*TARGET_GUILD_ID).unwrap_or(user.name);

            let res = format!(
                "**{}**: {} total, {} random, {} specific. favorite meme: *{}* ({})",
                username,
                info.random_memes + info.specific_memes,
                info.random_memes,
                info.specific_memes,
                info.most_used_meme,
                info.most_used_meme_count,
            );

            Ok(res)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .join("\n");

    send(msg.channel_id, &s, msg.tts)
}

pub fn query(_: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    use std::borrow::Borrow;

    use itertools::Itertools;
    use regex::Regex;
    use failure::err_msg;
    use serenity::model::id::UserId;

    use crate::{
        game::get_user_id,
        db,
        TARGET_GUILD_ID,
    };

    lazy_static! {
        static ref CREATOR_REGEX: Regex = Regex::new(r"(?i)(?:by|creator)=(.*)").unwrap();
        static ref AGE_REGEX: Regex = Regex::new(r"(?i)(?:age|order)=(.*)").unwrap();
    }

    let guild = msg.channel_id.to_channel()?
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild.read()
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild
        .read();

    let creator: Option<u64> = {
        let creator = args.current_quoted().map(|s| CREATOR_REGEX.is_match(s)).unwrap_or(false);
        if creator {
            args.single_quoted::<String>()
                .ok()
                .and_then(|s| CREATOR_REGEX.captures(&s).and_then(|c| c.get(1)).map(|x| x.as_str().to_owned()))
                .and_then(|s| get_user_id(guild.borrow(), s).ok().map(|s| s.0))
        } else {
            None
        }
    };

    let order = {
        let order = args.current_quoted().map(|s| AGE_REGEX.is_match(s)).unwrap_or(false);

        if order {
            args.single_quoted::<String>().ok()
                .and_then(|s| AGE_REGEX.captures(&s).and_then(|c| c.get(1)).map(|x| x.as_str().to_owned()))
                .map(|s| s.contains("new"))
                .unwrap_or(true)
        } else {
            true
        }
    };

    let result = db::query_meme(args.rest(), creator, order)?
        .into_iter()
        .map(|(meme, metadata)| {
            let user = UserId(metadata.created_by as u64).to_user()?;
            let username = user.nick_in(*TARGET_GUILD_ID).unwrap_or(user.name);

            Ok(format!("*{}* by **{}** ({}). text length: **{}**, image: **{}**, audio: **{}**",
                       meme.title,
                       username,
                       metadata.created.date().format(CLEAN_DATE_FORMAT),
                       meme.content.map_or(0, |s| s.len()),
                       meme.image_id.map_or("NO", |_s| "YES"),
                       meme.audio_id.map_or("NO", |_s| "YES"),
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .join("\n");

    if result.len() == 0 {
        info!("no memes matched query");
        return send(msg.channel_id, "no match".to_owned(), msg.tts);
    }

    send(msg.channel_id, &result, msg.tts)
}