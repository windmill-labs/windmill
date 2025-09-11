-- Add up migration script here
ALTER TABLE cloud_workspace_settings 
    ADD COLUMN is_past_due BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN max_tolerated_executions INTEGER;

CREATE OR REPLACE FUNCTION notify_team_plan_status_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.workspace_id); -- reuse the same channel as the one used for workspace premium change => clear cache
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER team_plan_status_change_trigger
AFTER UPDATE OF is_past_due, max_tolerated_executions ON cloud_workspace_settings
FOR EACH ROW
EXECUTE FUNCTION notify_team_plan_status_change();