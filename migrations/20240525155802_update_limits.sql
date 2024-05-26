-- Add migration script here
DROP TABLE faq;

CREATE TABLE faq (
    server_id BIGINT NOT NULL,
    title TEXT NOT NULL,
    contents TEXT,
    image TEXT,
    edit_time BIGINT NOT NULL,
    author BIGINT NOT NULL,
    link TEXT
)