CREATE TABLE invocation_records (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    meme_id INTEGER NOT NULL,
    time TIMESTAMP NOT NULL DEFAULT current_timestamp,
    random BOOLEAN NOT NULL
);

CREATE INDEX invocation_user_id ON invocation_records (user_id);
CREATE INDEX invocation_time ON invocation_records (time);
CREATE INDEX invocation_meme_random ON invocation_records (meme_id, random);
