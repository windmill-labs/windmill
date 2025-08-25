-- Add up migration script here
ALTER TABLE resource_type
  ADD COLUMN format_extension VARCHAR(20);
