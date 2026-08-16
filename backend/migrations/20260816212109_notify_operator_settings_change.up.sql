-- `operator_settings.builder` is an authorization decision (it gates flow and app writes for
-- operators) and is read through a per-process cache. Without this, revoking builder rights on one
-- API replica leaves every other replica authorizing writes until its own entry expires.
-- SECURITY DEFINER so the INSERT runs as the function owner: windmill_user fires this trigger and
-- has no rights on notify_event.
CREATE OR REPLACE FUNCTION notify_operator_settings_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES ('notify_operator_settings_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

DROP TRIGGER IF EXISTS operator_settings_change_trigger ON workspace_settings;
CREATE TRIGGER operator_settings_change_trigger
AFTER UPDATE OF operator_settings ON workspace_settings
FOR EACH ROW
EXECUTE FUNCTION notify_operator_settings_change();
