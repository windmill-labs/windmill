-- Add up migration script here
-- Add up migration script here

CREATE OR REPLACE FUNCTION notify_workspace_envs_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_envs_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER workspace_envs_change_trigger
AFTER INSERT OR UPDATE OF name, value OR DELETE ON workspace_env
FOR EACH ROW
EXECUTE FUNCTION notify_workspace_envs_change();
