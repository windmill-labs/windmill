-- Add up migration script here
ALTER TABLE http_trigger
ADD COLUMN workspaced_route BOOLEAN NOT NULL DEFAULT false;