ALTER TABLE text_memes ADD CONSTRAINT text_memes_content_not_all_null
    CHECK (content IS NOT NULL OR image_id IS NOT NULL OR audio_id IS NOT NULL);

ALTER TABLE text_memes ADD CONSTRAINT text_memes_image_or_audio_null
    CHECK (image_id IS NULL OR audio_id IS NULL);

ALTER TABLE text_memes ALTER COLUMN content DROP NOT NULL;

INSERT INTO text_memes(audio_id, metadata_id, title) SELECT audio_id, metadata_id, title FROM audio_memes;
INSERT INTO text_memes(image_id, metadata_id, title) SELECT image_id, metadata_id, title FROM image_memes;

DROP TABLE audio_memes;
DROP TABLE image_memes;

ALTER TABLE text_memes RENAME TO memes;

CREATE INDEX memes_audio    ON memes (audio_id);
CREATE INDEX memes_image    ON memes (image_id);
CREATE INDEX memes_content  ON memes (content);
