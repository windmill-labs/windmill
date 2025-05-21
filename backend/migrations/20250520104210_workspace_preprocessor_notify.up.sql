-- Add up migration script here
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