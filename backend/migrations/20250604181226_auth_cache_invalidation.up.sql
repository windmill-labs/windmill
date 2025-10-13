-- Add token invalidation notification trigger

CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    -- Only notify for session token deletions when the invalidation settings are enabled
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        PERFORM pg_notify('notify_token_invalidation', OLD.token);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER token_invalidation_trigger
AFTER DELETE ON token
FOR EACH ROW
EXECUTE FUNCTION notify_token_invalidation();
