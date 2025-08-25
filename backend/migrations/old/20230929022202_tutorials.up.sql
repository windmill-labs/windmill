-- Add up migration script here
CREATE TABLE  tutorial_progress(
    email VARCHAR(255) PRIMARY KEY,
    progress bit(64) NOT NULL DEFAULT B'0'
);

GRANT ALL ON tutorial_progress TO windmill_admin;
GRANT ALL ON tutorial_progress TO windmill_user;