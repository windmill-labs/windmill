-- Add up migration script here
ALTER TABLE http_trigger ADD COLUMN allowed_origins TEXT[];
