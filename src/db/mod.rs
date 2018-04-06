use std::env;

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

pub trait AssociatedData {
    type Associated;

    fn associated_data(&self, conn: &PgConnection) -> Result<Self::Associated>;
}

pub fn find_text(conn: &PgConnection, search: String) -> Result<TextMeme> {
    let format_search = format!("%{}%", search);

    text_memes::table
        .filter(text_memes::title.ilike(&format_search).or(text_memes::content.ilike(&format_search)))
        .limit(1)
        .first::<TextMeme>(conn)
        .map_err(Error::from)
}

pub fn find_audio(conn: &PgConnection, search: String) -> Result<AudioMeme> {
    let format_search = format!("%{}%", search);

    audio_memes::table
        .filter(audio_memes::title.ilike(format_search))
        .limit(1)
        .first::<AudioMeme>(conn)
        .map_err(Error::from)
}

pub fn find_image(conn: &PgConnection, search: String) -> Result<ImageMeme> {
    let format_search = format!("%{}%", search);

    image_memes::table
        .filter(image_memes::title.ilike(format_search))
        .limit(1)
        .first::<ImageMeme>(conn)
        .map_err(Error::from)
}

pub fn rand_text(conn: &PgConnection) -> Result<TextMeme> {
    text_memes::table
        .order(random.desc())
        .first::<TextMeme>(conn)
        .map_err(Error::from)
}

pub fn rand_image(conn: &PgConnection) -> Result<ImageMeme> {
    image_memes::table
        .order(random.desc())
        .first::<ImageMeme>(conn)
        .map_err(Error::from)
}

pub fn rand_audio(conn: &PgConnection) -> Result<AudioMeme> {
    audio_memes::table
        .order(random.desc())
        .first::<AudioMeme>(conn)
        .map_err(Error::from)
}

use diesel::sql_types;
no_arg_sql_function!(random, sql_types::Double, "SQL random() function");
