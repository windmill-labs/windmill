-- Add cc_token_url column for resource-level token URL override in OAuth client credentials flow
-- This allows resources to override the token URL from instance settings when using client credentials
ALTER TABLE account ADD COLUMN cc_token_url VARCHAR(500);
