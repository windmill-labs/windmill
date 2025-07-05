-- Rollback client_credentials OAuth flow support
ALTER TABLE account DROP COLUMN IF EXISTS grant_type;