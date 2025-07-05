-- Add support for client_credentials OAuth flow
-- Add grant_type column to distinguish between OAuth flows for backwards compatibility
ALTER TABLE account ADD COLUMN grant_type VARCHAR(50) NOT NULL DEFAULT 'authorization_code';