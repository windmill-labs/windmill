-- Make all notify_event trigger functions SECURITY DEFINER so that
-- INSERT INTO notify_event runs as the function owner (typically the
-- superuser that created the function) rather than the invoking role.
-- This prevents "permission denied for table notify_event" errors when
-- windmill_user or windmill_admin fire these triggers.

ALTER FUNCTION notify_config_change() SECURITY DEFINER;
ALTER FUNCTION notify_global_setting_change() SECURITY DEFINER;
ALTER FUNCTION notify_global_setting_delete() SECURITY DEFINER;
ALTER FUNCTION notify_webhook_change() SECURITY DEFINER;
ALTER FUNCTION notify_workspace_envs_change() SECURITY DEFINER;
ALTER FUNCTION notify_workspace_premium_change() SECURITY DEFINER;
ALTER FUNCTION notify_team_plan_status_change() SECURITY DEFINER;
ALTER FUNCTION notify_runnable_version_change() SECURITY DEFINER;
ALTER FUNCTION notify_http_trigger_change() SECURITY DEFINER;
ALTER FUNCTION notify_token_invalidation() SECURITY DEFINER;
ALTER FUNCTION notify_workspace_key_change() SECURITY DEFINER;
ALTER FUNCTION notify_workspace_rate_limit_change() SECURITY DEFINER;
