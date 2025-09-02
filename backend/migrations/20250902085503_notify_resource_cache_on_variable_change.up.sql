-- Add up migration script here
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
    END IF;
    PERFORM pg_notify('resource_cache_invalidation',
        json_build_object(
            'workspace_id', COALESCE(NEW.workspace_id, OLD.workspace_id),
            'path', COALESCE(NEW.path, OLD.path),
            'operation', TG_OP
        )::text
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;