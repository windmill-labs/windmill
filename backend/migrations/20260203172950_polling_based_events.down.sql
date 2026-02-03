-- Revert to pg_notify based event system

-- Restore notify_config_change function
CREATE OR REPLACE FUNCTION notify_config_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_config_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_global_setting_change function
CREATE OR REPLACE FUNCTION notify_global_setting_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_global_setting_delete function
CREATE OR REPLACE FUNCTION notify_global_setting_delete()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', OLD.name::text);
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_webhook_change function
CREATE OR REPLACE FUNCTION notify_webhook_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_webhook_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_workspace_envs_change function
CREATE OR REPLACE FUNCTION notify_workspace_envs_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_envs_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_workspace_premium_change function
CREATE OR REPLACE FUNCTION notify_workspace_premium_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_team_plan_status_change function
CREATE OR REPLACE FUNCTION notify_team_plan_status_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_runnable_version_change function
CREATE OR REPLACE FUNCTION notify_runnable_version_change()
RETURNS TRIGGER AS $$
DECLARE
    source_type TEXT;
    kind TEXT;
BEGIN
    source_type := TG_ARGV[0];

    IF source_type = 'script' THEN
        kind := NEW.kind;
    ELSE
        kind := 'flow';
    END IF;

    PERFORM pg_notify('notify_runnable_version_change', NEW.workspace_id || ':' || source_type || ':' || NEW.path || ':' || kind);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_http_trigger_change function
CREATE OR REPLACE FUNCTION notify_http_trigger_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_http_trigger_change', NEW.workspace_id || ':' || NEW.path);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_token_invalidation function
CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        PERFORM pg_notify('notify_token_invalidation', OLD.token);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_workspace_key_change function
CREATE OR REPLACE FUNCTION notify_workspace_key_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        PERFORM pg_notify('notify_workspace_key_change', OLD.workspace_id);
        RETURN OLD;
    ELSE
        PERFORM pg_notify('notify_workspace_key_change', NEW.workspace_id);
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Restore notify_var_resource_cache_change function
CREATE OR REPLACE FUNCTION notify_var_resource_cache_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_TABLE_NAME = 'variable' THEN
        PERFORM pg_notify('var_cache_invalidation',
            json_build_object(
                'workspace_id', COALESCE(NEW.workspace_id, OLD.workspace_id),
                'path', COALESCE(NEW.path, OLD.path),
                'operation', TG_OP
            )::text
        );
    ELSIF TG_TABLE_NAME = 'resource' THEN
        PERFORM pg_notify('resource_cache_invalidation',
            json_build_object(
                'workspace_id', COALESCE(NEW.workspace_id, OLD.workspace_id),
                'path', COALESCE(NEW.path, OLD.path),
                'operation', TG_OP
            )::text
        );
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Drop the notify_event table
DROP TABLE IF EXISTS notify_event;
