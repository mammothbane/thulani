CREATE TABLE tombstones (
    id          SERIAL PRIMARY KEY,
    meme_id     INTEGER NOT NULL,
    deleted_by  BIGINT NOT NULL,
    deleted_at  TIMESTAMP NOT NULL DEFAULT current_timestamp,
    metadata_id INTEGER REFERENCES metadata
);
