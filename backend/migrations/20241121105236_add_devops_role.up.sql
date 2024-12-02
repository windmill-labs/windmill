-- Add up migration script here
ALTER TABLE password ADD COLUMN devops BOOLEAN NOT NULL DEFAULT false;
