use std::env;

use diesel::prelude::*;

use super::{Result, Error};
pub use self::models::*;

mod schema;
mod models;

fn connection() -> Result<PgConnection> {
    let database_url = env::var("DATABASE_URL")?;
    PgConnection::establish(&database_url).map_err(Error::from)
}

pub fn find_text(search: String) -> Result<TextMeme> {
    use self::schema::text_memes::dsl::*;

    let format_search = format!("%{}%", search);

    let conn = connection()?;
    text_memes
        .filter(title.ilike(&format_search).or(content.ilike(&format_search)))
        .limit(1)
        .first::<TextMeme>(&conn)
        .map_err(Error::from)
}

pub fn find_audio(search: String) -> Result<AudioMeme> {
    use self::schema::audio_memes::dsl::*;

    let format_search = format!("%{}%", search);

    let conn = connection()?;
    audio_memes
        .filter(title.ilike(format_search))
        .limit(1)
        .first::<AudioMeme>(&conn)
        .map_err(Error::from)
}

pub fn rand_audio() -> Result<AudioMeme> {
    use self::schema::audio_memes::dsl::*;

    let conn = connection()?;
    audio_memes
        .order(random.desc())
        .first::<AudioMeme>(&conn)
        .map_err(Error::from)
}

pub fn rand_text() -> Result<TextMeme> {
    use self::schema::text_memes::dsl::*;

    let conn = connection()?;
    text_memes
        .order(random.desc())
        .first::<TextMeme>(&conn)
        .map_err(Error::from)
}

use diesel::sql_types;
no_arg_sql_function!(random, sql_types::Double, "SQL random() function");
