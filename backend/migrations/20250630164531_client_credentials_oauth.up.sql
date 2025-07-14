-- Add support for client_credentials OAuth flow
-- Add grant_type column to distinguish between OAuth flows for backwards compatibility
ALTER TABLE account ADD COLUMN grant_type VARCHAR(50) NOT NULL DEFAULT 'authorization_code';

-- Add client_id and client_secret columns for resource-level client credentials
-- These are used when grant_type = 'client_credentials' and user wants to use resource-level credentials
-- instead of instance-level credentials configured by admin
ALTER TABLE account
ADD COLUMN cc_client_id VARCHAR(500),
ADD COLUMN cc_client_secret VARCHAR(500);
