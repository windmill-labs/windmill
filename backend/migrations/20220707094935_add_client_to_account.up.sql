-- Add up migration script here
ALTER TABLE account ADD COLUMN client VARCHAR(50);
ALTER TABLE resource ADD COLUMN is_oauth BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE variable ADD COLUMN is_oauth BOOLEAN NOT NULL DEFAULT false;
