-- note: pgcrypto extension must exist for this database

CREATE UNIQUE INDEX audio_data ON audio     (digest(data, 'sha1'));
CREATE UNIQUE INDEX image_data ON images    (digest(data, 'sha1'));
