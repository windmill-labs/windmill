-- Add up migration script here
ALTER TABLE http_trigger
ADD workspaced_route boolean NOT NULL DEFAULT false;