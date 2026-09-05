-- Additive fixture: a second workspace sharing the `base` fixture's admin user,
-- so the same caller can hold an app in each workspace.

INSERT INTO workspace (id, name, owner) VALUES
	('test-workspace-2', 'test-workspace-2', 'test-user');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace-2', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace-2', 'cloud', 'test-key-2');

INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace-2');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace-2', 'all', 'All users', '{}');
