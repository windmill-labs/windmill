-- Hashing is irreversible so the values will be stale hashes, but the old
-- code's DELETE FROM token WHERE token = $1 is non-fatal — refresh tokens
-- keep working, only old access token cleanup silently fails.
ALTER TABLE mcp_oauth_refresh_token RENAME COLUMN access_token_hash TO access_token;
