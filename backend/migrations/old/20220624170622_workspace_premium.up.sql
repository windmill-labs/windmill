-- Add up migration script here
ALTER TABLE workspace ADD COLUMN premium BOOLEAN NOT NULL DEFAULT false;
