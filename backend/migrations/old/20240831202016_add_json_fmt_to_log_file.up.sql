-- Add up migration script here
ALTER TABLE log_file ADD COLUMN IF NOT EXISTS json_fmt boolean DEFAULT false;