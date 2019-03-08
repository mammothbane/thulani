CREATE TABLE google_oauth_tokens (
    id SERIAL PRIMARY KEY,
    token VARCHAR NOT NULL,
    refresh_token VARCHAR NOT NULL,
    expiration TIMESTAMP NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT current_timestamp
);

CREATE INDEX oauth_token_exp_index ON google_oauth_tokens (expiration);
CREATE INDEX oauth_token_created_index ON google_oauth_tokens (created);
