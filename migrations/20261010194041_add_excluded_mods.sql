-- Add migration script here
CREATE TABLE excluded_mods (
    server_id BIGINT NOT NULL,
    mod_name TEXT NOT NULL
);