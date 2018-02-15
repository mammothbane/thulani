table! {
    audio_memes (id) {
        id -> Int4,
        title -> Varchar,
        link -> Varchar,
    }
}

table! {
    text_memes (id) {
        id -> Int4,
        title -> Varchar,
        content -> Text,
        pic_related -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    audio_memes,
    text_memes,
);
