ALTER TABLE mods RENAME TO mods_old;
CREATE TABLE mods (
    name TEXT PRIMARY KEY NOT NULL,
    title TEXT,
    owner TEXT NOT NULL,
    summary TEXT,
    category TEXT NOT NULL,
    downloads_count INT NOT NULL,
    factorio_version TEXT,
    version TEXT,
    released_at INT NOT NULL
);
INSERT INTO mods (name, title, owner, summary, category, downloads_count, factorio_version, version, released_at) SELECT name, title, owner, summary, category, downloads_count, factorio_version, version, released_at FROM mods_old;
DROP TABLE mods_old;

ALTER TABLE servers RENAME TO servers_old;
CREATE TABLE servers (
    server_id BIGINT PRIMARY KEY NOT NULL,
    updates_channel BIGINT,
    modrole BIGINT,
    show_changelog BOOLEAN
);
INSERT INTO servers (server_id, updates_channel, modrole, show_changelog) SELECT server_id, updates_channel, modrole, show_changelog FROM servers_old;
DROP TABLE servers_old;

ALTER TABLE subscribed_mods RENAME TO subscribed_mods_old;
CREATE TABLE subscribed_mods (
    server_id BIGINT NOT NULL,
    mod_name TEXT NOT NULL
);
INSERT INTO subscribed_mods (server_id, mod_name) SELECT server_id, mod_name FROM subscribed_mods_old;
DROP TABLE subscribed_mods_old;

ALTER TABLE subscribed_authors RENAME TO subscribed_authors_old;
CREATE TABLE subscribed_authors (
    server_id BIGINT,
    author_name TEXT
);
INSERT INTO subscribed_authors (server_id, author_name) SELECT server_id, author_name FROM subscribed_authors_old;
DROP TABLE subscribed_authors_old;