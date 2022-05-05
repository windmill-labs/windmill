-- Add up migration script here
ALTER TABLE schedule DROP COLUMN script_hash;
ALTER TABLE schedule ADD COLUMN is_flow boolean NOT NULL DEFAULT false;
ALTER TABLE schedule ALTER COLUMN script_path SET NOT NULL;;
