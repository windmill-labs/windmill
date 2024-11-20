INSERT INTO flow_version (workspace_id, path, value, schema, created_by, created_at) 
SELECT workspace_id, path, value, schema, edited_by, edited_at 
	FROM flow 
	WHERE NOT EXISTS (SELECT 1 FROM flow_version WHERE flow_version.workspace_id = flow.workspace_id AND flow_version.path = flow.path);
UPDATE flow
SET versions = subquery.versions
FROM (
	SELECT
		path,
		workspace_id,
		array_agg(id ORDER BY created_at ASC) AS versions
	FROM
		flow_version
	GROUP BY
		path,
		workspace_id
) subquery
WHERE
	flow.path = subquery.path
	AND flow.workspace_id = subquery.workspace_id;