use super::schema::*;
use chrono::naive::NaiveDateTime;

#[derive(Insertable, Queryable, Identifiable, AsChangeset, Debug, Associations)]
#[belongs_to(Audio)]
#[belongs_to(Image)]
#[belongs_to(TextMeme)]
#[belongs_to(ImageMeme)]
#[belongs_to(TextMeme)]
#[table_name="metadata"]
pub struct Metadata {
    pub id: i32,
    pub created: NaiveDateTime,
    pub created_by: i64,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug, Associations)]
#[belongs_to(AudioMeme)]
#[belongs_to(TextMeme)]
#[table_name="audio"]
pub struct Audio {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug, Associations)]
#[belongs_to(ImageMeme)]
#[belongs_to(TextMeme)]
#[table_name="images"]
pub struct Image {
    pub id: i32,
    pub data: Vec<u8>,
    pub metadata_id: i32,
}

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug)]
#[table_name="audio_memes"]
pub struct AudioMeme {
    pub id: i32,
    pub title: String,
    pub audio_id: i32,
    pub metadata_id: i32,
}

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

#[derive(Insertable, Queryable, Identifiable, PartialEq, AsChangeset, Debug, Associations)]
#[belongs_to(Metadata)]
#[table_name="audit_records"]
pub struct AuditRecord {
    pub id: i32,
    pub updated: NaiveDateTime,
    pub updated_by: i64,
    pub metadata_id: i32,
}
