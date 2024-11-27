-- Add down migration script here
DROP TABLE capture_config;
DELETE FROM capture;
ALTER TABLE capture DROP CONSTRAINT capture_pkey;
ALTER TABLE capture DROP COLUMN is_flow, DROP COLUMN trigger_kind, DROP COLUMN trigger_extra, DROP COLUMN id;
ALTER TABLE capture ADD CONSTRAINT capture_pkey PRIMARY KEY (workspace_id, path);
DROP TYPE TRIGGER_KIND;
