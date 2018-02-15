use super::schema::*;

#[derive(Insertable, Queryable, Identifiable, AsChangeset)]
#[table_name="audio_memes"]
pub struct AudioMeme {
    pub id: i32,
    pub title: String,
    pub link: String,
}

#[derive(Insertable, Queryable, Identifiable, AsChangeset)]
#[table_name="text_memes"]
pub struct TextMeme {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub pic_related: String,
}
