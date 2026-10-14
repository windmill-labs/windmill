-- Layers on `base`: a second workspace the superadmin `test-user` is also a member of, so a
-- share can be requested through the wrong workspace's path by a caller the route accepts.

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
