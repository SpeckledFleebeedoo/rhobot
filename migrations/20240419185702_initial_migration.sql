-- Add migration script here
CREATE TABLE mods (
    name TEXT PRIMARY KEY,
    title TEXT,
    owner TEXT,
    summary TEXT,
    category TEXT,
    downloads_count INT,
    factorio_version TEXT,
    version TEXT,
    released_at INT
);

CREATE TABLE servers (
    server_id BIGINT PRIMARY KEY,
    updates_channel BIGINT,
    modrole BIGINT,
    show_changelog BOOLEAN
);

CREATE TABLE subscribed_mods (
    server_id BIGINT,
    mod_name TEXT
);

CREATE TABLE subscribed_authors (
    server_id BIGINT,
    author_name TEXT
);

CREATE TABLE faq (
    server_id BIGINT,
    title TEXT,
    contents TEXT,
    image TEXT,
    edit_time INT,
    author INT,
    link TEXT
);