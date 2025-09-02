-- Add up migration script here

-- Create notification function for variable and resource cache invalidation
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

-- Create triggers for variable table
CREATE TRIGGER variable_cache_invalidate_trigger
    AFTER INSERT OR UPDATE OR DELETE ON variable
    FOR EACH ROW EXECUTE FUNCTION notify_var_resource_cache_change();

-- Create triggers for resource table  
CREATE TRIGGER resource_cache_invalidate_trigger
    AFTER INSERT OR UPDATE OR DELETE ON resource
    FOR EACH ROW EXECUTE FUNCTION notify_var_resource_cache_change();