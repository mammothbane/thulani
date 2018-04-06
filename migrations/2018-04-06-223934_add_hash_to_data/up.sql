ALTER TABLE audio   ADD COLUMN data_hash bytea;
ALTER TABLE images  ADD COLUMN data_hash bytea;

UPDATE audio    SET data_hash = digest(data, 'sha1');
UPDATE images   SET data_hash = digest(data, 'sha1');

ALTER TABLE audio   ADD CONSTRAINT audio_hash_valid CHECK (data_hash = digest(data, 'sha1'));
ALTER TABLE images  ADD CONSTRAINT image_hash_valid CHECK (data_hash = digest(data, 'sha1'));

ALTER TABLE audio   ALTER COLUMN data_hash SET NOT NULL;
ALTER TABLE images  ALTER COLUMN data_hash SET NOT NULL;

CREATE UNIQUE INDEX audio_hash on audio     (data_hash);
CREATE UNIQUE INDEX image_hash on images    (data_hash);

DROP INDEX audio_data;
DROP INDEX image_data;
