-- Deleting an MCP OAuth access token, by whatever path, revokes the refresh grant it was
-- issued with and evicts it from the auth cache. An expired token is left alone: the expiry
-- sweep deletes it, and its grant must stay refreshable.
CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', OLD.token_prefix);
    ELSIF OLD.label LIKE 'mcp-oauth-%' AND COALESCE(OLD.expiration > now(), true) THEN
        UPDATE mcp_oauth_refresh_token SET revoked = TRUE WHERE access_token_hash = OLD.token_hash;
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', OLD.token_prefix);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE INDEX IF NOT EXISTS idx_mcp_oauth_refresh_token_access_token_hash
    ON mcp_oauth_refresh_token(access_token_hash);
