use std::{
    convert::AsRef,
    env,
};

use chrono::{
    DateTime,
    Utc,
};
use diesel::{
    NotFound,
    prelude::*,
    r2d2::{ConnectionManager, ManageConnection},
};

use crate::{Error, Result};

pub use self::models::*;
use self::schema::*;

mod schema;
mod models;

lazy_static! {
    static ref DB_URL: String = env::var("DATABASE_URL").expect("no database url in environment").into();
    static ref CONN_MGR: ConnectionManager<PgConnection> = ConnectionManager::new(DB_URL.clone());
}

pub fn connection() -> Result<PgConnection> {
    CONN_MGR.connect().map_err(Error::from)
}

pub fn find_meme<T: AsRef<str>>(conn: &PgConnection, search: T) -> Result<Meme> {
    use diesel::dsl::sql;
    use diesel::sql_types::Text;

    let search = search.as_ref();

    // TODO: check for injection
    let mut meme = memes::table
        .filter(memes::title.eq(search))
        .limit(1)
        .first::<Meme>(conn);

    if let Err(NotFound) = meme {
        let format_search = format!("%{}%", search);

        meme = memes::table
            .filter(memes::title.ilike(&format_search).or(sql("content ILIKE ").bind::<Text, _>(&format_search)))
            .limit(1)
            .first::<Meme>(conn);
    }

    meme
        .map_err(Error::from)
}

pub fn delete_meme<T: AsRef<str>>(conn: &PgConnection, search: T, deleted_by: u64) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let deleted = memes::table
            .filter(memes::title.eq(search.as_ref()))
            .first::<Meme>(conn)?;

        ::diesel::delete(memes::table)
            .filter(memes::id.eq(deleted.id))
            .execute(conn)?;

        if let Some(image_id) = deleted.image_id {
            let count = memes::table
                .filter(memes::image_id.eq(image_id))
                .count()
                .execute(conn)?;

            if count == 0 {
                ::diesel::delete(images::table)
                    .filter(images::id.eq(image_id))
                    .execute(conn)?;
            }
        }

        if let Some(audio_id) = deleted.audio_id {
            let count = memes::table
                .select(::diesel::dsl::count_star())
                .filter(memes::audio_id.eq(audio_id))
                .execute(conn)?;

            if count == 0 {
                ::diesel::delete(audio::table)
                    .filter(audio::id.eq(audio_id))
                    .execute(conn)?;
            }
        }

        let tombstone = NewTombstone {
            deleted_by: deleted_by as i64,
            metadata_id: deleted.metadata_id,
            meme_id: deleted.id,
        };

        let _ = ::diesel::insert_into(tombstones::table)
            .values(&tombstone)
            .execute(conn)?;

        Ok(())
    })
}

pub fn rand_meme(conn: &PgConnection, audio: bool) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
    use failure::err_msg;
    use std::ops::Try;

    let ids: Vec<i32> = if audio {
        memes::table
            .select(memes::id)
            .filter(memes::content.is_not_null()
                .or(memes::image_id.is_not_null())
                .or(memes::audio_id.is_not_null()))
            .load(conn)
            .map_err(Error::from)?
    } else {
        memes::table
            .select(memes::id)
            .filter(memes::content.is_not_null()
                .or(memes::image_id.is_not_null()))
            .load(conn)
            .map_err(Error::from)?
    };

    let id = ids.choose(&mut thread_rng())
        .into_result()
        .map_err( |_| err_msg("couldn't load meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_audio_meme(conn: &PgConnection) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
    use failure::err_msg;
    use std::ops::Try;

    let ids: Vec<i32> = memes::table
        .select(memes::id)
        .filter(memes::audio_id.is_not_null())
        .load(conn)
        .map_err(Error::from)?;

    let id = ids.choose(&mut thread_rng())
        .into_result()
        .map_err(|_| err_msg("couldn't load audio meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_silent_meme(conn: &PgConnection) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
    use failure::err_msg;
    use std::ops::Try;

    let ids: Vec<i32> = memes::table
        .select(memes::id)
        .filter(memes::audio_id.is_null())
        .load(conn)
        .map_err(Error::from)?;

    let id = ids.choose(&mut thread_rng())
        .into_result()
        .map_err(|_| err_msg("couldn't load audio meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

#[derive(Debug, Copy, Clone)]
pub struct Stats {
    pub memes_overall: usize,
    pub audio_memes: usize,
    pub image_memes: usize,
    pub started_recording: DateTime<Utc>,
    pub total_meme_invocations: usize,
    pub audio_meme_invocations: usize,
    pub random_meme_invocations: usize,
}

pub fn stats(conn: &PgConnection) -> Result<Stats> {
    use diesel::dsl::{count_star, count};
    use chrono::NaiveDateTime;

    let total_count: i64 = memes::table
        .select(count_star())
        .first(conn)
        .map_err(Error::from)?;

    let image_count: i64 = memes::table
        .select(count(memes::image_id))
        .filter(memes::image_id.is_not_null())
        .first(conn)
        .map_err(Error::from)?;

    let audio_count: i64 = memes::table
        .select(count(memes::audio_id))
        .filter(memes::audio_id.is_not_null())
        .first(conn)
        .map_err(Error::from)?;

    let started_recording: NaiveDateTime = invocation_records::table
        .select(invocation_records::time)
        .order(invocation_records::time)
        .first(conn)
        .map_err(Error::from)?;

    let started_recording = DateTime::from_utc(started_recording, Utc{});

    let total_meme_invocations: i64 = invocation_records::table
        .select(count_star())
        .first(conn)
        .map_err(Error::from)?;

    let audio_meme_invocations: i64 = invocation_records::table
        .inner_join(memes::table)
        .select(count_star())
        .filter(memes::audio_id.is_not_null())
        .first(conn)
        .map_err(Error::from)?;

    let random_meme_invocations: i64 = invocation_records::table
        .select(count_star())
        .filter(invocation_records::random.eq(true))
        .first(conn)
        .map_err(Error::from)?;

    Ok(Stats {
        memes_overall: total_count as usize,
        image_memes: image_count as usize,
        audio_memes: audio_count as usize,
        started_recording,
        total_meme_invocations: total_meme_invocations as usize,
        audio_meme_invocations: audio_meme_invocations as usize,
        random_meme_invocations: random_meme_invocations as usize,
    })
}
