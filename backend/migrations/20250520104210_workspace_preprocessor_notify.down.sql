-- Add down migration script here
CREATE OR REPLACE FUNCTION notify_runnable_version_change()
RETURNS TRIGGER AS $$
DECLARE
    source_type TEXT;
BEGIN
    source_type := TG_ARGV[0];

    PERFORM pg_notify('notify_runnable_version_change', NEW.workspace_id || ':' || source_type || ':' || NEW.path);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;