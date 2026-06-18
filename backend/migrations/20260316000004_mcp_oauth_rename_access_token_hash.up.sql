-- Hash existing plaintext access_token values in mcp_oauth_refresh_token.
-- Only hash rows that are not already 64-char hex strings (safety guard for re-runs).
UPDATE mcp_oauth_refresh_token
SET access_token = encode(sha256(access_token::bytea), 'hex');

ALTER TABLE mcp_oauth_refresh_token RENAME COLUMN access_token TO access_token_hash;
