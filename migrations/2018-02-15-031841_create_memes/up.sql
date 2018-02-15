CREATE TABLE text_memes (
    id          SERIAL PRIMARY KEY,
    title       varchar UNIQUE NOT NULL,
    content     TEXT NOT NULL,
    pic_related varchar NULL,
    UNIQUE(content, pic_related)
);

CREATE TABLE audio_memes (
    id      SERIAL PRIMARY KEY,
    title   varchar UNIQUE NOT NULL,
    link    varchar UNIQUE NOT NULL
);

