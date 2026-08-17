-- Invalidate auth cache (across instances) when token scopes change.
-- Reuses the existing notify_token_invalidation channel handled in main.rs.

CREATE OR REPLACE FUNCTION notify_token_scopes_change()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.scopes IS DISTINCT FROM NEW.scopes THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', NEW.token_prefix);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER token_scopes_update_trigger
AFTER UPDATE OF scopes ON token
FOR EACH ROW
EXECUTE FUNCTION notify_token_scopes_change();
