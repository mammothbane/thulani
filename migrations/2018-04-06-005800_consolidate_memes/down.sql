DROP INDEX memes_audio;
DROP INDEX memes_content;
DROP INDEX memes_image;

ALTER TABLE memes RENAME TO text_memes;

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


INSERT INTO audio_memes(title, audio_id, metadata_id) SELECT title, audio_id, metadata_id FROM text_memes
    WHERE audio_id IS NOT NULL AND content IS NULL;

DELETE FROM text_memes WHERE audio_id IS NOT NULL AND content IS NULL;


INSERT INTO image_memes(title, image_id, metadata_id) SELECT title, image_id, metadata_id FROM text_memes
    WHERE image_id IS NOT NULL AND content IS NULL;

DELETE FROM text_memes WHERE image_id IS NOT NULL AND content IS NULL;


ALTER TABLE text_memes ALTER COLUMN content SET NOT NULL;
ALTER TABLE text_memes DROP CONSTRAINT text_memes_image_or_audio_null;
ALTER TABLE text_memes DROP CONSTRAINT text_memes_content_not_all_null;
