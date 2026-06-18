ALTER TYPE asset_kind ADD VALUE 'datatable';

ALTER TABLE workspace_settings
ADD COLUMN datatable JSONB;