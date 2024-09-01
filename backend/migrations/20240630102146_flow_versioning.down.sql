-- Add down migration script here
DROP INDEX deployment_metadata_flow;
ALTER TABLE deployment_metadata DROP COLUMN flow_version;
create index if not exists deployment_metadata_flow on deployment_metadata (workspace_id, path) where script_hash is null and app_version is null;

alter table flow drop column versions;
DROP TABLE flow_version;