-- Add up migration script here
ALTER TABLE http_trigger ADD COLUMN static_asset_config JSONB;