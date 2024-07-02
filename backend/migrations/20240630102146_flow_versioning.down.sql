-- Add down migration script here
DROP INDEX deployment_metadata_flow;
ALTER TABLE deployment_metadata DROP COLUMN flow_version;
create index if not exists deployment_metadata_flow on deployment_metadata (workspace_id, path) where script_hash is null and app_version is null;

UPDATE flow SET value = fv.value, schema = fv.schema, edited_by = fv.created_by, edited_at = fv.created_at FROM flow_version fv WHERE fv.id = flow.versions[array_upper(flow.versions, 1)];
ALTER TABLE flow alter column value set not null, alter column edited_by set not null, alter column edited_at set not null;
alter table flow drop column versions;
DROP TABLE flow_version;