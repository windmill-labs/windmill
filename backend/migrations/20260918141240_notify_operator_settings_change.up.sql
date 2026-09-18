-- The write rights in `operator_settings` are authorization decisions read through a per-process
-- cache. Without this, withdrawing one on a single API replica leaves every other replica
-- authorizing writes until its own entry expires.
CREATE OR REPLACE FUNCTION notify_operator_settings_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES ('notify_operator_settings_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS operator_settings_change_trigger ON workspace_settings;
CREATE TRIGGER operator_settings_change_trigger
AFTER UPDATE OF operator_settings ON workspace_settings
FOR EACH ROW
WHEN (OLD.operator_settings IS DISTINCT FROM NEW.operator_settings)
EXECUTE FUNCTION notify_operator_settings_change();
