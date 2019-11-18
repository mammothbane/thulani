use std::{
    convert::AsRef,
    env,
};

use chrono::{
    Date,
    DateTime,
    Utc,
};
use diesel::{
    NotFound,
    prelude::*,
    r2d2::{ConnectionManager, ManageConnection},
};
use postgres::Connection as RawPgConn;
use r2d2_postgres::{
    PostgresConnectionManager as RawPgConnMgr,
    TlsMode,
};

use anyhow::anyhow;
use lazy_static::lazy_static;

use crate::{Error, Result};

pub use self::models::*;
use self::schema::*;

mod schema;
mod models;

lazy_static! {
    static ref DB_URL: String = env::var("DATABASE_URL").expect("no database url in environment").into();
    static ref CONN_MGR: ConnectionManager<PgConnection> = ConnectionManager::new(DB_URL.clone());
    static ref RAW_CONN_MGR: RawPgConnMgr = RawPgConnMgr::new(DB_URL.clone(), TlsMode::None).unwrap();
}

#[inline]
pub fn connection() -> Result<PgConnection> {
    CONN_MGR.connect().map_err(Error::from)
}

#[inline]
fn raw_connection() -> Result<RawPgConn> {
    RAW_CONN_MGR.connect().map_err(Error::from)
}

pub fn find_meme<T: AsRef<str>>(conn: &PgConnection, search: T) -> Result<Meme> {
    use diesel::dsl::sql;
    use diesel::sql_types::Text;

    let search = search.as_ref();

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

pub fn query_meme<T: AsRef<str>>(search: T, user_id: Option<u64>, age_desc: bool) -> Result<Vec<(Meme, Metadata)>> {
    let raw_conn = raw_connection()?;

    let search = format!("%{}%", search.as_ref());

    let rows = raw_conn.query(&format!(r#"
    SELECT memes.id, title, content, image_id, audio_id, metadata_id, created, created_by
    FROM memes
    INNER JOIN metadata ON memes.metadata_id = metadata.id
    WHERE (memes.title ILIKE $1 OR memes.content ILIKE $1)
              AND (metadata.created_by = $2 OR $3)
    ORDER BY metadata.created {}
    LIMIT 100
    "#,
        if age_desc { "DESC" } else { "ASC" },
    ), &[
        &search,
        &(user_id.unwrap_or(0) as i64),
        &user_id.is_none(),
    ])?;

    let result = rows.iter()
        .map(|row| {
            let meme = Meme {
                id: row.get(0),
                title: row.get(1),
                content: row.get(2),
                image_id: row.get(3),
                audio_id: row.get(4),
                metadata_id: row.get(5),
            };

            let metadata = Metadata {
                id: row.get(5),
                created: row.get(6),
                created_by: row.get(7),
            };

            (meme, metadata)
        })
        .collect();

    Ok(result)
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

pub fn rare_meme(conn: &PgConnection, audio: bool) -> Result<Meme> {
    use rand::prelude::*;

    let raw_conn = raw_connection()?;

    let rows = raw_conn.query(r#"
    WITH
    meme_count AS (
        SELECT
               meme_id,
               COUNT(*) AS ct
        FROM invocation_records
        GROUP BY meme_id
    ),
    aggregate AS (
        SELECT
               memes.id AS meme_id,
               COALESCE(meme_count.ct, 0) AS ct,
               EXTRACT(EPOCH FROM (now() - metadata.created)) AS time_diff
        FROM meme_count
            RIGHT JOIN memes ON memes.id = meme_count.meme_id
            INNER JOIN metadata ON metadata.id = memes.metadata_id
        WHERE (memes.audio_id IS NULL) = $1 OR $2
    ),
    least_used AS (
        SELECT
               meme_id,
               TRUNC(time_diff / (ct + 1)) as play_prop
        FROM aggregate
    )
    SELECT
           meme_id,
           sum(play_prop) OVER (ORDER BY play_prop DESC) as play_prop
    FROM least_used
    LIMIT 100;
    "#, &[&!audio, &audio])?;

    let elems = rows.iter()
        .map(|row| (row.get::<_, i32>(0), row.get::<_, f64>(1) as i64))
        .collect::<Vec<_>>();

    if elems.len() == 0 {
        return Err(anyhow!("no rare memes found"));
    }

    let mut rng = thread_rng();
    let target_prob = rng.gen_range(0, elems.last().unwrap().1);

    let meme_id = elems.into_iter()
        .find(|(_, x)| target_prob < *x)
        .ok_or(anyhow!("couldn't locate meme satisfying target probability"))?
        .0;

    Meme::find(conn, meme_id)
}

pub fn rand_meme(conn: &PgConnection, audio: bool) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
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
        .map_err( |_| anyhow!("couldn't load meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_audio_meme(conn: &PgConnection) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
    use std::ops::Try;

    let ids: Vec<i32> = memes::table
        .select(memes::id)
        .filter(memes::audio_id.is_not_null())
        .load(conn)
        .map_err(Error::from)?;

    let id = ids.choose(&mut thread_rng())
        .into_result()
        .map_err(|_| anyhow!("couldn't load audio meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_silent_meme(conn: &PgConnection) -> Result<Meme> {
    use rand::{thread_rng, seq::SliceRandom};
    use std::ops::Try;

    let ids: Vec<i32> = memes::table
        .select(memes::id)
        .filter(memes::audio_id.is_null())
        .load(conn)
        .map_err(Error::from)?;

    let id = ids.choose(&mut thread_rng())
        .into_result()
        .map_err(|_| anyhow!("couldn't load audio meme"))?;

    memes::table
        .find(id)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub memes_overall: usize,
    pub audio_memes: usize,
    pub image_memes: usize,
    pub started_recording: DateTime<Utc>,
    pub total_meme_invocations: usize,
    pub audio_meme_invocations: usize,
    pub random_meme_invocations: usize,

    pub most_active_day: Date<Utc>,
    pub most_active_day_count: usize,

    pub most_audio_active_day: Date<Utc>,
    pub most_audio_active_count: usize,

    pub most_random_meme_user: u64,
    pub most_random_meme_user_count: usize,
    pub most_directly_named_meme_user: u64,
    pub most_directly_named_meme_count: usize,

    pub most_popular_named_meme: String,
    pub most_popular_named_meme_count: usize,

    pub most_popular_random_meme: String,
    pub most_popular_random_meme_count: usize,

    pub most_popular_meme_overall: String,
    pub most_popular_meme_overall_count: usize,
}

pub fn stats(conn: &PgConnection) -> Result<Stats> {
    use diesel::dsl::{count_star, count};
    use chrono::{
        NaiveDateTime,
        NaiveDate,
    };

    #[inline]
    fn to_utc(ndt: NaiveDateTime) -> DateTime<Utc> {
        DateTime::from_utc(ndt, Utc{})
    }

    #[inline]
    fn to_utc_date(nd: NaiveDate) -> Date<Utc> {
        Date::from_utc(nd, Utc{})
    }

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

    let started_recording = to_utc(started_recording);

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

    let raw_conn = raw_connection()?;

    let rows = raw_conn.query(r#"
    SELECT DATE(time) as dt, COUNT(*) FROM invocation_records
    GROUP BY dt
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_active_day = to_utc_date(row.get(0));
    let most_active_day_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT DATE(time) as dt, COUNT(*) FROM invocation_records
    INNER JOIN memes ON invocation_records.meme_id = memes.id
    WHERE memes.audio_id IS NOT NULL
    GROUP BY dt
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_active_audio_day = to_utc_date(row.get(0));
    let most_active_audio_day_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT user_id, COUNT(*) FROM invocation_records
    WHERE random IS TRUE
    GROUP BY user_id
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_random_invoker: i64 = row.get(0);
    let most_random_invoker_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT user_id, COUNT(*) FROM invocation_records
    WHERE random IS FALSE
    GROUP BY user_id
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_specific_invoker: i64 = row.get(0);
    let most_specific_invoker_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT memes.title, COUNT(*) FROM invocation_records
    INNER JOIN memes ON meme_id = memes.id
    WHERE random IS FALSE
    GROUP BY memes.title
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_requested_meme = row.get(0);
    let most_requested_meme_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT memes.title, COUNT(*) FROM invocation_records
    INNER JOIN memes ON meme_id = memes.id
    WHERE random IS TRUE
    GROUP BY memes.title
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_random_meme = row.get(0);
    let most_random_meme_count: i64 = row.get(1);

    let rows = raw_conn.query(r#"
    SELECT memes.title, COUNT(*) FROM invocation_records
    INNER JOIN memes ON meme_id = memes.id
    GROUP BY memes.title
    ORDER BY COUNT(*) DESC
    LIMIT 1;
    "#, &[])?;

    let row = rows.get(0);

    let most_invoked_meme = row.get(0);
    let most_invoked_meme_count: i64 = row.get(1);

    Ok(Stats {
        memes_overall: total_count as usize,
        image_memes: image_count as usize,
        audio_memes: audio_count as usize,
        started_recording,
        total_meme_invocations: total_meme_invocations as usize,
        audio_meme_invocations: audio_meme_invocations as usize,
        random_meme_invocations: random_meme_invocations as usize,

        most_active_day,
        most_active_day_count: most_active_day_count as usize,
        most_audio_active_day: most_active_audio_day,
        most_audio_active_count: most_active_audio_day_count as usize,

        most_random_meme_user: most_random_invoker as u64,
        most_random_meme_user_count: most_random_invoker_count as usize,
        most_directly_named_meme_user: most_specific_invoker as u64,
        most_directly_named_meme_count: most_specific_invoker_count as usize,

        most_popular_named_meme: most_requested_meme,
        most_popular_named_meme_count: most_requested_meme_count as usize,

        most_popular_random_meme: most_random_meme,
        most_popular_random_meme_count: most_random_meme_count as usize,

        most_popular_meme_overall: most_invoked_meme,
        most_popular_meme_overall_count: most_invoked_meme_count as usize,
    })
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct MemerInfo {
    pub user_id: u64,
    pub random_memes: usize,
    pub specific_memes: usize,
    pub most_used_meme: String,
    pub most_used_meme_count: usize,
}

pub fn memers() -> Result<Vec<MemerInfo>> {
    let raw_conn = raw_connection()?;

    let rows = raw_conn.query(r#"
    WITH random_count AS (
        SELECT user_id, COUNT(*) as count
        FROM invocation_records
        WHERE random = TRUE
        GROUP BY user_id
    ),
         specific_count AS (
             SELECT user_id, COUNT(*) as count
             FROM invocation_records
             WHERE random = FALSE
             GROUP BY user_id
         ),
         user_meme_counts AS (
             SELECT user_id, meme_id, COUNT(meme_id) as meme_count
             FROM invocation_records
             WHERE EXISTS (SELECT id FROM memes WHERE id = invocation_records.meme_id)
             GROUP BY user_id, meme_id
             ORDER BY user_id, meme_count DESC
         ),
         most_memed_per_user AS (
             SELECT user_id, MAX(meme_count) as max_count
             FROM user_meme_counts
             GROUP BY user_id
         ),
         most_memed AS (
             SELECT DISTINCT ON (user_meme_counts.user_id) user_meme_counts.user_id, user_meme_counts.meme_id, user_meme_counts.meme_count
             FROM user_meme_counts
             INNER JOIN most_memed_per_user ON user_meme_counts.user_id = most_memed_per_user.user_id
             WHERE user_meme_counts.meme_count = most_memed_per_user.max_count
         )
    SELECT random_count.user_id, random_count.count, specific_count.count, memes.title, most_memed.meme_count
    FROM random_count
    INNER JOIN most_memed ON most_memed.user_id = random_count.user_id
    INNER JOIN specific_count ON specific_count.user_id = random_count.user_id
    INNER JOIN memes ON memes.id = most_memed.meme_id
    ORDER BY (random_count.count + specific_count.count) DESC
    "#, &[])?;

    let result = rows.iter().map(|row| {
        let user_id: i64 = row.get(0);
        let random_count: i64 = row.get(1);
        let specific_count: i64 = row.get(2);
        let most_memed_meme: String = row.get(3);
        let most_memed_count: i64 = row.get(4);

        MemerInfo {
            user_id: user_id as u64,
            random_memes: random_count as usize,
            specific_memes: specific_count as usize,
            most_used_meme: most_memed_meme,
            most_used_meme_count: most_memed_count as usize,
        }
    })
        .collect();

    Ok(result)
}
