-- Add down migration script here
DROP FUNCTION notify_team_plan_status_change;
DROP TRIGGER notify_team_plan_status_change ON cloud_workspace_settings;

ALTER TABLE cloud_workspace_settings 
    DROP COLUMN is_past_due,
    DROP COLUMN max_tolerated_executions;