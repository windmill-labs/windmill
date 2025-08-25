-- Add down migration script here
ALTER TABLE resource_type
  DROP COLUMN format_extension;
