use chrono::naive::NaiveDateTime;
use diesel::prelude::*;

use super::schema::*;
use super::AssociatedData;
use ::{Result, Error};

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="text_memes"]
pub struct TextMeme {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub image_id: Option<i32>,
    pub audio_id: Option<i32>,
    pub metadata_id: i32,
}

impl AssociatedData for TextMeme {
    type Associated = (Option<Image>, Option<Audio>);

    fn associated_data(&self, conn: &PgConnection) -> Result<Self::Associated> {
        let image = self.image_id.map(|x: i32| images::table.find(x).first(conn)).transpose()?;
        let audio = self.audio_id.map(|x: i32| audio::table.find(x).first(conn)).transpose()?;

        Ok((image, audio))
    }
}


#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="image_memes"]
pub struct ImageMeme {
    pub id: i32,
    pub title: String,
    pub image_id: i32,
    pub metadata_id: i32,
}

impl AssociatedData for ImageMeme {
    type Associated = Image;

    fn associated_data(&self, conn: &PgConnection) -> Result<Self::Associated> {
        images::table.find(self.image_id).first(conn).map_err(Error::from)
    }
}


#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="audio_memes"]
pub struct AudioMeme {
    pub id: i32,
    pub title: String,
    pub audio_id: i32,
    pub metadata_id: i32,
}

impl AssociatedData for AudioMeme {
    type Associated = Audio;

    fn associated_data(&self, conn: &PgConnection) -> Result<Self::Associated> {
        audio::table.find(self.audio_id).first(conn).map_err(Error::from)
    }
}


#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="audio"]
pub struct Audio {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="images"]
pub struct Image {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="metadata"]
pub struct Metadata {
    pub id: i32,
    pub created: NaiveDateTime,
    pub created_by: i64,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="audit_records"]
pub struct AuditRecord {
    pub id: i32,
    pub updated: NaiveDateTime,
    pub updated_by: i64,
    pub metadata_id: i32,
}
