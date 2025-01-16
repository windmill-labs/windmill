-- Add down migration script here
DROP FUNCTION v2_queue_instead_of_update_overlay() CASCADE;
DROP FUNCTION v2_queue_instead_of_update() CASCADE;
DROP FUNCTION v2_queue_instead_of_delete() CASCADE;
DROP FUNCTION v2_queue_update(OLD v2_queue, NEW v2_queue) CASCADE;
DROP VIEW v2_queue;
