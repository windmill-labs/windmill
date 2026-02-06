-- Create notify_event table for polling-based event system
-- This replaces PostgreSQL LISTEN/NOTIFY with a table-based approach

CREATE TABLE IF NOT EXISTS notify_event (
    id BIGSERIAL PRIMARY KEY,
    channel TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS notify_event_created_at_idx ON notify_event (created_at);

-- Drop redundant index if it exists (id is already the PRIMARY KEY)
DROP INDEX IF EXISTS notify_event_id_idx;

-- Update notify_config_change function
CREATE OR REPLACE FUNCTION notify_config_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_config_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_global_setting_change function
CREATE OR REPLACE FUNCTION notify_global_setting_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_global_setting_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_global_setting_delete function
CREATE OR REPLACE FUNCTION notify_global_setting_delete()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_global_setting_change', OLD.name::text);
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Update notify_webhook_change function
CREATE OR REPLACE FUNCTION notify_webhook_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_webhook_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_workspace_envs_change function
CREATE OR REPLACE FUNCTION notify_workspace_envs_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_workspace_envs_change', COALESCE(NEW.workspace_id, OLD.workspace_id));
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Update notify_workspace_premium_change function
CREATE OR REPLACE FUNCTION notify_workspace_premium_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_workspace_premium_change', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_team_plan_status_change function
CREATE OR REPLACE FUNCTION notify_team_plan_status_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_workspace_premium_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_runnable_version_change function
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

    INSERT INTO notify_event (channel, payload) VALUES ('notify_runnable_version_change', NEW.workspace_id || ':' || source_type || ':' || NEW.path || ':' || kind);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update notify_http_trigger_change function
CREATE OR REPLACE FUNCTION notify_http_trigger_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_http_trigger_change', COALESCE(NEW.workspace_id, OLD.workspace_id) || ':' || COALESCE(NEW.path, OLD.path));
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Update notify_token_invalidation function
CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        INSERT INTO notify_event (channel, payload) VALUES ('notify_token_invalidation', OLD.token);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Update notify_workspace_key_change function
CREATE OR REPLACE FUNCTION notify_workspace_key_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        INSERT INTO notify_event (channel, payload) VALUES ('notify_workspace_key_change', OLD.workspace_id);
        RETURN OLD;
    ELSE
        INSERT INTO notify_event (channel, payload) VALUES ('notify_workspace_key_change', NEW.workspace_id);
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- NOTE: var_cache_invalidation / resource_cache_invalidation triggers were
-- intentionally dropped in migration 20250902085504. We do NOT re-create them
-- here to keep this migration scoped to the LISTEN/NOTIFY → polling swap only.
