-- Rollback cc_token_url column addition
ALTER TABLE account DROP COLUMN IF EXISTS cc_token_url;
