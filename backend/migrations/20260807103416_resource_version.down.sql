-- Add down migration script here

DROP TRIGGER IF EXISTS record_resource_version_insert_trigger ON resource;
DROP TRIGGER IF EXISTS record_resource_version_update_trigger ON resource;
DROP FUNCTION IF EXISTS record_resource_version();
DROP TABLE resource_version;
