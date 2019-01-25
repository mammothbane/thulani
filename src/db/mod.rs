use std::{
    env,
    convert::AsRef,
};

use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, ManageConnection},
    NotFound,
};

use super::{Result, Error};
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

pub fn rand_text(conn: &PgConnection) -> Result<Meme> {
    memes::table
        .filter(memes::content.is_not_null())
        .order(random.desc())
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_image(conn: &PgConnection) -> Result<Meme> {
    memes::table
        .filter(memes::image_id.is_not_null())
        .order(random.desc())
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn rand_audio(conn: &PgConnection) -> Result<Meme> {
    memes::table
        .filter(memes::audio_id.is_not_null())
        .order(random.desc())
        .first::<Meme>(conn)
        .map_err(Error::from)
}

use diesel::sql_types;
no_arg_sql_function!(random, sql_types::Double, "SQL random() function");
