ALTER TABLE images  ADD COLUMN filename VARCHAR;

UPDATE images SET filename = 'unknown.bin';

ALTER TABLE images  ALTER COLUMN filename SET NOT NULL;
