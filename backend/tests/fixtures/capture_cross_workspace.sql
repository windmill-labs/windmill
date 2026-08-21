-- Fixture for the cross-workspace capture deletion regression test.
-- `test-user-2` is a plain (non-admin) member of test-workspace only, so RLS
-- applies. test-workspace-2 holds a capture under `u/test-user-2/...`: the
-- capture policies key on the path segment alone, so that path is inside the
-- member's grants in *every* workspace.

INSERT INTO workspace (id, name, owner) VALUES
	('test-workspace', 'test-workspace', 'test-user'),
	('test-workspace-2', 'test-workspace-2', 'test-user');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key'),
	('test-workspace-2', 'cloud', 'test-key-2');

INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace'), ('test-workspace-2');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'all', 'All users', '{}'),
	('test-workspace-2', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('test@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Test User', 'test-user');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('test2@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 2');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin'),
	('test-workspace-2', 'test@windmill.dev', 'test-user', true, 'Admin'),
	('test-workspace', 'test2@windmill.dev', 'test-user-2', false, 'User');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
	(encode(sha256('SECRET_TOKEN_2'::bytea), 'hex'), 'SECRET_TOK', 'SECRET_TOKEN_2', 'test2@windmill.dev', 'test token 2', false);

INSERT INTO capture (id, workspace_id, path, created_by, main_args, is_flow, trigger_kind) VALUES
	(1, 'test-workspace-2', 'u/test-user-2/victim', 'test-user', '{"secret": "other workspace payload"}'::jsonb, false, 'webhook'),
	(2, 'test-workspace',   'u/test-user-2/own',    'test-user-2', '{}'::jsonb, false, 'webhook');

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;
