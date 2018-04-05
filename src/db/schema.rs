table! {
    audio (id) {
        id -> Int4,
        data -> Bytea,
        metadata_id -> Int4,
    }
}

table! {
    audio_memes (id) {
        id -> Int4,
        title -> Varchar,
        audio_id -> Int4,
        metadata_id -> Int4,
    }
}

table! {
    audit_records (id) {
        id -> Int4,
        updated -> Timestamp,
        updated_by -> Int8,
        metadata_id -> Int4,
    }
}

table! {
    image_memes (id) {
        id -> Int4,
        title -> Varchar,
        image_id -> Int4,
        metadata_id -> Int4,
    }
}

table! {
    images (id) {
        id -> Int4,
        data -> Bytea,
        metadata_id -> Int4,
    }
}

table! {
    metadata (id) {
        id -> Int4,
        created -> Timestamp,
        created_by -> Int8,
    }
}

table! {
    text_memes (id) {
        id -> Int4,
        title -> Varchar,
        content -> Text,
        image_id -> Nullable<Int4>,
        audio_id -> Nullable<Int4>,
        metadata_id -> Int4,
    }
}

joinable!(audio -> metadata (metadata_id));
joinable!(audio_memes -> audio (audio_id));
joinable!(audio_memes -> metadata (metadata_id));
joinable!(audit_records -> metadata (metadata_id));
joinable!(image_memes -> images (image_id));
joinable!(image_memes -> metadata (metadata_id));
joinable!(images -> metadata (metadata_id));
joinable!(text_memes -> audio (audio_id));
joinable!(text_memes -> images (image_id));
joinable!(text_memes -> metadata (metadata_id));

allow_tables_to_appear_in_same_query!(
    audio,
    audio_memes,
    audit_records,
    image_memes,
    images,
    metadata,
    text_memes,
);
