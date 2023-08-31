-- Add up migration script here
ALTER TABLE script ADD COLUMN cache_ttl INTEGER;
ALTER TABLE queue ADD COLUMN cache_ttl INTEGER;
