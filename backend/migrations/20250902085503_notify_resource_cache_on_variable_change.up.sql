-- Add up migration script here
DROP TRIGGER variable_cache_invalidate_trigger ON variable;
DROP TRIGGER resource_cache_invalidate_trigger ON resource;
DROP FUNCTION notify_var_resource_cache_change();