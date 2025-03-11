-- Add up migration script here
ALTER TABLE http_trigger
ADD COLUMN wrap_body BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN raw_string BOOLEAN NOT NULL DEFAULT false;