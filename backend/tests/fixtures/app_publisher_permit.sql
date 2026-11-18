-- A publisher-mode app owned by `test-user`, not shared with anyone, whose one
-- runnable is a deployed script.

INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES
    ('test-workspace', 'test-user', 'export async function main() { return "ok" }', '{}', '', '', 'u/test-user/hello', 7788101, 'deno', '');

INSERT INTO app (id, workspace_id, path, summary, policy, versions) VALUES
	(999101, 'test-workspace', 'u/test-user/papp', 'publisher app',
	 '{"execution_mode": "publisher", "on_behalf_of": "u/test-user", "on_behalf_of_email": "test@windmill.dev",
	   "triggerables_v2": {"c:script/u/test-user/hello": {"static_inputs": {}, "one_of_inputs": {}}}}'::jsonb, '{}');
