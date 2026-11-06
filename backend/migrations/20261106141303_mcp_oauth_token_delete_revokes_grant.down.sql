DROP INDEX IF EXISTS idx_mcp_oauth_refresh_token_access_token_hash;

CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', OLD.token_prefix);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
