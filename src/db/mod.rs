use std::env;
use std::convert::AsRef;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, ManageConnection};

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

    let search = search.as_ref();
    let format_search = format!("%{}%", search);

    memes::table
        .filter(memes::title.ilike(&format_search).or(sql(&format!("content ILIKE %{}%", search))))
        .limit(1)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn find_text<T: AsRef<str>>(conn: &PgConnection, search: T) -> Result<Meme> {
    use diesel::dsl::sql;

    let search = search.as_ref();
    let format_search = format!("%{}%", search);

    memes::table
        .filter((memes::title.ilike(&format_search).or(sql(&format!("content ILIKE %{}%", search))))
            .and(memes::content.is_not_null()))
        .limit(1)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn find_audio<T: AsRef<str>>(conn: &PgConnection, search: T) -> Result<Meme> {
    let format_search = format!("%{}%", search.as_ref());

    memes::table
        .filter(memes::title.ilike(format_search).and(memes::audio_id.is_not_null()))
        .limit(1)
        .first::<Meme>(conn)
        .map_err(Error::from)
}

pub fn find_image<T: AsRef<str>>(conn: &PgConnection, search: T) -> Result<Meme> {
    let format_search = format!("%{}%", search.as_ref());

    memes::table
        .filter(memes::title.ilike(format_search).and(memes::image_id.is_not_null()))
        .limit(1)
        .first::<Meme>(conn)
        .map_err(Error::from)
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
