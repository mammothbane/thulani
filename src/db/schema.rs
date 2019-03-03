table! {
    audio (id) {
        id -> Int4,
        data -> Bytea,
        metadata_id -> Int4,
        data_hash -> Bytea,
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
    images (id) {
        id -> Int4,
        data -> Bytea,
        metadata_id -> Int4,
        data_hash -> Bytea,
        filename -> Varchar,
    }
}

table! {
    invocation_records (id) {
        id -> Int4,
        user_id -> Int8,
        message_id -> Int8,
        meme_id -> Int4,
        time -> Timestamp,
        random -> Bool,
    }
}

table! {
    memes (id) {
        id -> Int4,
        title -> Varchar,
        content -> Nullable<Text>,
        image_id -> Nullable<Int4>,
        audio_id -> Nullable<Int4>,
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
    tombstones (id) {
        id -> Int4,
        meme_id -> Int4,
        deleted_by -> Int8,
        deleted_at -> Timestamp,
        metadata_id -> Nullable<Int4>,
    }
}

joinable!(audio -> metadata (metadata_id));
joinable!(audit_records -> metadata (metadata_id));
joinable!(images -> metadata (metadata_id));
joinable!(memes -> audio (audio_id));
joinable!(memes -> images (image_id));
joinable!(memes -> metadata (metadata_id));
joinable!(tombstones -> metadata (metadata_id));

allow_tables_to_appear_in_same_query!(
    audio,
    audit_records,
    images,
    invocation_records,
    memes,
    metadata,
    tombstones,
);
