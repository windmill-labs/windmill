-- Add up migration script here
ALTER TABLE
    password
ADD
    first_time_user boolean NOT NULL DEFAULT (false);

UPDATE
    password
SET
    first_time_user = true
WHERE
    email = 'admin@windmill.dev';