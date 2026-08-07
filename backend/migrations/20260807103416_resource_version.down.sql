-- Add down migration script here

DROP TRIGGER IF EXISTS record_resource_version_trigger ON resource;
DROP FUNCTION IF EXISTS record_resource_version();
DROP TABLE resource_version;
