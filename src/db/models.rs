use chrono::naive::NaiveDateTime;
use diesel::prelude::*;

use super::schema::*;
use ::{Result, Error};

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
        self.image_id.map(|x: i32| images::table.find(x).first(conn).map_err(Error::from))
    }

    pub fn audio(&self, conn: &PgConnection) -> Option<Result<Audio>> {
        self.audio_id.map(|x: i32| audio::table.find(x).first(conn).map_err(Error::from))
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


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="audio"]
pub struct Audio {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="audio"]
pub struct NewAudio {
    pub data: Vec<u8>,
    pub metadata_id: i32,
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="images"]
pub struct Image {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="images"]
pub struct NewImage {
    pub data: Vec<u8>,
    pub metadata_id: i32,
}


#[derive(Queryable, Identifiable, PartialEq, Debug)]
#[table_name="metadata"]
pub struct Metadata {
    pub id: i32,
    pub created: NaiveDateTime,
    pub created_by: i64,
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name="metadata"]
pub struct NewMetadata {
    pub created: NaiveDateTime,
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

#[derive(Insertable, PartialEq, Debug)]
#[table_name="audit_records"]
pub struct NewAuditRecord {
    pub updated: NaiveDateTime,
    pub updated_by: i64,
    pub metadata_id: i32,
}
