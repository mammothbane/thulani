CREATE TABLE metadata (
    id          SERIAL PRIMARY KEY,

    created     TIMESTAMP NOT NULL DEFAULT current_timestamp,
    created_by  BIGINT NOT NULL
);

CREATE INDEX metadata_created on metadata (created);
CREATE INDEX metadata_created_by on metadata (created_by);


CREATE TABLE audit_records (
    id          SERIAL PRIMARY KEY,

    updated     TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_by  BIGINT NOT NULL,

    metadata_id INTEGER REFERENCES metadata NOT NULL
);

CREATE INDEX audit_updated on audit_records (updated);
CREATE INDEX audit_updated_by on audit_records (updated_by);
CREATE INDEX audit_metadata on audit_records (metadata_id);

CREATE INDEX audit_metadata_updated_by on audit_records (metadata_id, updated_by);


CREATE TABLE images (
    id    SERIAL PRIMARY KEY,
    data  bytea NOT NULL,

    metadata_id INTEGER REFERENCES metadata UNIQUE NOT NULL
);


CREATE TABLE audio (
    id    SERIAL PRIMARY KEY,
    data  bytea NOT NULL,

    metadata_id INTEGER REFERENCES metadata UNIQUE NOT NULL
);


CREATE TABLE text_memes (
    id          SERIAL PRIMARY KEY,
    title       varchar UNIQUE NOT NULL,
    content     TEXT NOT NULL,
    image_id    INTEGER REFERENCES images NULL,
    audio_id    INTEGER REFERENCES audio NULL,

    metadata_id INTEGER REFERENCES metadata UNIQUE NOT NULL,
    UNIQUE(content, image_id, audio_id)
);


CREATE TABLE image_memes (
    id          SERIAL PRIMARY KEY,
    title       varchar UNIQUE NOT NULL,
    image_id    INTEGER REFERENCES images NOT NULL,

    metadata_id INTEGER REFERENCES metadata UNIQUE NOT NULL,
    UNIQUE(title, image_id)
);


CREATE TABLE audio_memes (
    id          SERIAL PRIMARY KEY,
    title       varchar UNIQUE NOT NULL,
    audio_id    INTEGER REFERENCES audio NOT NULL,

    metadata_id INTEGER REFERENCES metadata UNIQUE NOT NULL,
    UNIQUE(title, audio_id)
);
