-- Add up migration script here

-- create flow_version table with index
CREATE TABLE flow_version (
  id bigserial PRIMARY KEY,
	workspace_id varchar(50) NOT NULL,
	path varchar(255) NOT NULL,
	value jsonb,
	schema json,
	created_by varchar(50) NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	FOREIGN KEY (workspace_id, path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE
);
CREATE INDEX index_flow_version_path_created_at ON flow_version (path, created_at);


-- add versions column to flow
ALTER TABLE flow ADD COLUMN versions bigint[] NOT NULL DEFAULT '{}'::bigint[];
-- create flow_version records for existing flows and update flow versions
INSERT INTO flow_version (workspace_id, path, value, schema, created_by, created_at) SELECT workspace_id, path, value, schema, edited_by, edited_at FROM flow;
UPDATE flow
SET versions = subquery.versions
FROM (
	SELECT
		path,
		workspace_id,
		array_agg(id) AS versions
	FROM
		flow_version
	GROUP BY
		path,
		workspace_id
) subquery
WHERE
	flow.path = subquery.path
	AND flow.workspace_id = subquery.workspace_id;

-- add flow_version column to deployment_metadata
ALTER TABLE deployment_metadata ADD COLUMN flow_version int8;
-- populate flow_version column in deployment_metadata
UPDATE deployment_metadata
SET flow_version = fv.id
FROM flow_version fv
WHERE deployment_metadata.workspace_id = fv.workspace_id
AND deployment_metadata.path = fv.path
AND deployment_metadata.app_version IS NULL AND deployment_metadata.script_hash IS NULL;
-- update flow metadata index to include flow_verison
DROP INDEX IF EXISTS deployment_metadata_flow;
CREATE UNIQUE INDEX IF NOT EXISTS deployment_metadata_flow ON deployment_metadata (workspace_id, path, flow_version) WHERE flow_version IS NOT NULL;

-- make sure the windmill_user and windmill_admin roles have access to the new tables
GRANT ALL ON flow_version TO windmill_user;
GRANT ALL ON flow_version_id_seq TO windmill_user;
GRANT ALL ON flow_version TO windmill_admin;
GRANT ALL ON flow_version_id_seq TO windmill_admin;