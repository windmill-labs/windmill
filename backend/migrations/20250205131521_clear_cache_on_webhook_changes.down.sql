-- Add down migration script here

DROP TRIGGER webhook_change_trigger ON workspace_settings;
DROP FUNCTION notify_webhook_change();
