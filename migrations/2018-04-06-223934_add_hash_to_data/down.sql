CREATE UNIQUE INDEX audio_data on audio     (digest(data, 'sha1'));
CREATE UNIQUE INDEX image_data on images    (digest(data, 'sha1'));

DROP INDEX audio_hash;
DROP INDEX image_hash;

ALTER TABLE audio   DROP CONSTRAINT audio_hash_valid;
ALTER TABLE images  DROP CONSTRAINT image_hash_valid;

ALTER TABLE audio DROP COLUMN data_hash;
ALTER TABLE images DROP COLUMN data_hash;
