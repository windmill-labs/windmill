-- Layers on `base`, which already provides test-workspace and `test-user-2`, a
-- plain non-admin member of it (token SECRET_TOKEN_2).
--
-- Adds a second workspace holding a capture under `u/test-user-2/…`: the capture
-- policies key on the path segment alone, so that path is inside the member's
-- grants in *every* workspace.

INSERT INTO workspace (id, name, owner) VALUES
	('test-workspace-2', 'test-workspace-2', 'test-user');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace-2', 'cloud', 'test-key-2');

INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace-2');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace-2', 'all', 'All users', '{}');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace-2', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO capture (id, workspace_id, path, created_by, main_args, is_flow, trigger_kind) VALUES
	(1, 'test-workspace-2', 'u/test-user-2/victim', 'test-user', '{"secret": "other workspace payload"}'::jsonb, false, 'webhook'),
	(2, 'test-workspace',   'u/test-user-2/own',    'test-user-2', '{}'::jsonb, false, 'webhook');
