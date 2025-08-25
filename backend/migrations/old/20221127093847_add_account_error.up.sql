-- Add up migration script here
ALTER TABLE account ADD COLUMN refresh_error TEXT;
