-- Add up migration script here
ALTER TABLE app ADD COLUMN workspaced_route BOOLEAN NOT NULL DEFAULT false;