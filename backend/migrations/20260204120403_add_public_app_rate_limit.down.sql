DROP TRIGGER IF EXISTS workspace_rate_limit_change_trigger ON workspace_settings;
DROP FUNCTION IF EXISTS notify_workspace_rate_limit_change();

ALTER TABLE workspace_settings
DROP COLUMN IF EXISTS public_app_execution_limit_per_minute;
