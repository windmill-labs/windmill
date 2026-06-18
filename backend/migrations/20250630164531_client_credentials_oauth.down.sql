-- Rollback client_credentials OAuth flow support
ALTER TABLE account DROP COLUMN IF EXISTS grant_type;
ALTER TABLE account DROP COLUMN IF EXISTS cc_client_id;
ALTER TABLE account DROP COLUMN IF EXISTS cc_client_secret;
