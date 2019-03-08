use chrono::naive::NaiveDateTime;
use diesel::prelude::*;

use crate::{
    db::schema::*,
    Error,
    Result,
};

#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="memes"]
pub struct Meme {
    pub id: i32,
    pub title: String,
    pub content: Option<String>,
    pub image_id: Option<i32>,
    pub audio_id: Option<i32>,
    pub metadata_id: i32,
}

impl Meme {
    pub fn image(&self, conn: &PgConnection) -> Option<Result<Image>> {
        self.image_id.map(|x: i32| images::table.filter(images::id.eq(x)).first(conn).map_err(Error::from))
    }

    pub fn audio(&self, conn: &PgConnection) -> Option<Result<Audio>> {
        self.audio_id.map(|x: i32| audio::table.filter(audio::id.eq(x)).first(conn).map_err(Error::from))
    }

    pub fn find(conn: &PgConnection, id: i32) -> Result<Meme> {
        memes::table.find(id).get_result(conn).map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="memes"]
pub struct NewMeme {
    pub title: String,
    pub content: Option<String>,
    pub image_id: Option<i32>,
    pub audio_id: Option<i32>,
    pub metadata_id: i32,
}

impl NewMeme {
    pub fn save(mut self, conn: &PgConnection, by_user: u64) -> Result<Meme> {
        let metadata = Metadata::create(conn, by_user)?;

        self.metadata_id = metadata.id;

        ::diesel::insert_into(memes::table)
            .values(&self)
            .get_result::<Meme>(conn)
            .map_err(Error::from)
    }
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="audio"]
pub struct Audio {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
    pub data_hash: Vec<u8>,
}

impl Audio {
    pub fn create(conn: &PgConnection, data: Vec<u8>, by_user: u64) -> Result<i32> {
        let mut data_hash = ::sha1::Sha1::new();
        data_hash.update(&data);
        let data_hash = data_hash.digest().bytes().to_vec();

        let id = audio::table
            .select(audio::id)
            .filter(audio::data_hash.eq(&data_hash))
            .get_results::<i32>(conn)?;

        if let Some(id) = id.first() {
            return Ok(*id);
        }

        let metadata = Metadata::create(conn, by_user)?;

        let new_audio = NewAudio {
            data,
            data_hash,
            metadata_id: metadata.id,
        };

        ::diesel::insert_into(audio::table)
            .values(&new_audio)
            .returning(audio::id)
            .get_result(conn)
            .map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="audio"]
pub struct NewAudio {
    pub data: Vec<u8>,
    pub metadata_id: i32,
    pub data_hash: Vec<u8>,
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="images"]
pub struct Image {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
    pub data_hash: Vec<u8>,
    pub filename: String,
}

impl Image {
    pub fn create(conn: &PgConnection, filename: &str, data: Vec<u8>, by_user: u64) -> Result<i32> {
        let mut data_hash = ::sha1::Sha1::new();
        data_hash.update(&data);
        let data_hash = data_hash.digest().bytes().to_vec();

        let id = images::table
            .select(images::id)
            .filter(images::data_hash.eq(&data_hash))
            .get_results::<i32>(conn)?;

        if let Some(id) = id.first() {
            return Ok(*id);
        }

        let metadata = Metadata::create(conn, by_user)?;

        let new_image = NewImage {
            data,
            data_hash,
            filename: filename.to_owned(),
            metadata_id: metadata.id,
        };

        ::diesel::insert_into(images::table)
            .values(&new_image)
            .returning(images::id)
            .get_result(conn)
            .map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="images"]
pub struct NewImage {
    pub data: Vec<u8>,
    pub metadata_id: i32,
    pub data_hash: Vec<u8>,
    pub filename: String,
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="metadata"]
pub struct Metadata {
    pub id: i32,
    pub created: NaiveDateTime,
    pub created_by: i64,
}

impl Metadata {
    pub fn create(conn: &PgConnection, by_user: u64) -> Result<Metadata> {
        ::diesel::insert_into(metadata::table)
            .values(&NewMetadata {
                created_by: by_user as i64,
            })
            .get_result::<Metadata>(conn)
            .map_err(Error::from)
    }

    pub fn find(conn: &PgConnection, id: i32) -> Result<Metadata> {
        metadata::table.find(id)
            .get_result::<Metadata>(conn)
            .map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="metadata"]
pub struct NewMetadata {
    pub created_by: i64,
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="audit_records"]
pub struct AuditRecord {
    pub id: i32,
    pub updated: NaiveDateTime,
    pub updated_by: i64,
    pub metadata_id: i32,
}

impl AuditRecord {
    pub fn create(conn: &PgConnection, metadata: i32, by_user: u64) -> Result<AuditRecord> {
        ::diesel::insert_into(audit_records::table)
            .values(&NewAuditRecord {
                updated_by: by_user as i64,
                metadata_id: metadata,
            })
            .get_result::<AuditRecord>(conn)
            .map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="audit_records"]
pub struct NewAuditRecord {
    pub updated_by: i64,
    pub metadata_id: i32,
}

#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="tombstones"]
pub struct Tombstone {
    pub id: i32,
    pub deleted: NaiveDateTime,
    pub deleted_by: i64,
    pub metadata_id: i32,
    pub meme_id: i32,
}


#[derive(Insertable, PartialEq, Debug)]
#[table_name="tombstones"]
pub struct NewTombstone {
    pub deleted_by: i64,
    pub metadata_id: i32,
    pub meme_id: i32,
}

#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="invocation_records"]
pub struct InvocationRecord {
    pub id: i32,
    pub user_id: i64,
    pub message_id: i64,
    pub meme_id: i32,
    pub time: NaiveDateTime,
    pub random: bool,
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="invocation_records"]
pub struct NewInvocationRecord {
    pub user_id: i64,
    pub message_id: i64,
    pub meme_id: i32,
    pub random: bool,
}

impl InvocationRecord {
    pub fn create(conn: &PgConnection, user_id: u64, message_id: u64, meme_id: i32, random: bool) -> Result<Self> {
        ::diesel::insert_into(invocation_records::table)
            .values(&NewInvocationRecord {
                user_id: user_id as i64,
                message_id: message_id as i64,
                meme_id,
                random,
            })
            .get_result::<InvocationRecord>(conn)
            .map_err(Error::from)
    }

    pub fn last(conn: &PgConnection) -> Result<Self> {
        invocation_records::table
            .order(invocation_records::time.desc())
            .first(conn)
            .map_err(Error::from)
    }

    pub fn last_n(conn: &PgConnection, n: usize) -> Result<Vec<Self>> {
        invocation_records::table
            .order(invocation_records::time.desc())
            .limit(n as i64)
            .load(conn)
            .map_err(Error::from)
    }
}

#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="google_oauth_tokens"]
pub struct GoogleOAuthToken {
    pub id: i32,
    pub token: String,
    pub refresh_token: String,
    pub expiration: NaiveDateTime,
    pub created: NaiveDateTime,
}

impl GoogleOAuthToken {
    pub fn create(conn: &PgConnection, token: String, refresh_token: String, expiration: NaiveDateTime) -> Result<Self> {
        ::diesel::insert_into(google_oauth_tokens::table)
            .values(&NewGoogleOAuthToken {
                token,
                refresh_token,
                expiration,
            })
            .get_result::<GoogleOAuthToken>(conn)
            .map_err(Error::from)
    }

    pub fn latest(conn: &PgConnection) -> Result<Self> {
        use chrono;

        let now = chrono::Utc::now().naive_utc();

        google_oauth_tokens::table
            .filter(google_oauth_tokens::expiration.gt(now))
            .order(google_oauth_tokens::created.desc())
            .first(conn)
            .map_err(Error::from)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="google_oauth_tokens"]
pub struct NewGoogleOAuthToken {
    pub token: String,
    pub refresh_token: String,
    pub expiration: NaiveDateTime,
}
