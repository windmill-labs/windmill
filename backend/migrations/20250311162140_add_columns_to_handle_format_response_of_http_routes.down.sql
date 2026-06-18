-- Add down migration script here
ALTER TABLE http_trigger
DROP COLUMN wrap_body,
DROP COLUMN raw_string;