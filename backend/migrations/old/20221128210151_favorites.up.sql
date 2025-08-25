-- Add up migration script here

CREATE TYPE FAVORITE_KIND AS ENUM ('app', 'script', 'flow');

CREATE TABLE favorite (
    usr VARCHAR(50) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    favorite_kind FAVORITE_KIND NOT NULL,
    PRIMARY KEY (usr, workspace_id, favorite_kind, path)
);