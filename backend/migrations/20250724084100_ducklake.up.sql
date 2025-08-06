ALTER TYPE asset_kind ADD VALUE 'ducklake';

ALTER TABLE workspace_settings
ADD COLUMN ducklake JSONB;
