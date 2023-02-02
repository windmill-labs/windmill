-- Add up migration script here
ALTER TABLE account ALTER COLUMN refresh_token TYPE VARCHAR(1500);
