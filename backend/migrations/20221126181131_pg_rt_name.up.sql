-- Add up migration script here
UPDATE resource_type rt SET name = 'postgresql' WHERE name = 'postgres' AND  NOT EXISTS (
   SELECT 1 FROM resource_type WHERE name = 'postgresql' AND rt.workspace_id = workspace_id
);

UPDATE resource SET resource_type = 'postgresql' WHERE resource_type = 'postgres';
