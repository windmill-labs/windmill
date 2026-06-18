ALTER TABLE workspace_settings
ADD COLUMN IF NOT EXISTS public_app_execution_limit_per_minute INTEGER DEFAULT NULL;

-- Add trigger function for rate limit changes
CREATE OR REPLACE FUNCTION notify_workspace_rate_limit_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES ('notify_workspace_rate_limit_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger on workspace_settings (drop first if exists)
DROP TRIGGER IF EXISTS workspace_rate_limit_change_trigger ON workspace_settings;
CREATE TRIGGER workspace_rate_limit_change_trigger
AFTER UPDATE OF public_app_execution_limit_per_minute ON workspace_settings
FOR EACH ROW
EXECUTE FUNCTION notify_workspace_rate_limit_change();
