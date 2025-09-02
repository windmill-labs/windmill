-- Add up migration script here
DROP TRIGGER IF EXISTS variable_cache_invalidate_trigger ON variable;
DROP TRIGGER IF EXISTS resource_cache_invalidate_trigger ON resource;
DROP FUNCTION IF EXISTS notify_var_resource_cache_change();