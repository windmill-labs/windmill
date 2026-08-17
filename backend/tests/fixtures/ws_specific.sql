-- Fixture for ws_specific integration tests.
-- Adds a non-admin user (test-user-2) as a regular workspace member with
-- access to user-owned items at u/test-user-2/* but not u/test-user/*.

INSERT INTO workspace
            (id,               name,             owner)
     VALUES ('test-workspace', 'test-workspace', 'test-user')
ON CONFLICT DO NOTHING;

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin')
ON CONFLICT DO NOTHING;

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test2@windmill.dev', 'test-user-2', false, 'User')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace')
ON CONFLICT DO NOTHING;

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'all', 'All users', '{}')
ON CONFLICT DO NOTHING;

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('test@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Test User', 'test-user')
ON CONFLICT DO NOTHING;

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('test2@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 2')
ON CONFLICT DO NOTHING;

-- Tokens
insert INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
    (encode(sha256('SECRET_TOKEN'::bytea), 'hex'), 'SECRET_TOK', 'SECRET_TOKEN', 'test@windmill.dev', 'admin token', true)
ON CONFLICT DO NOTHING;

insert INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
    (encode(sha256('SECRET_TOKEN_2'::bytea), 'hex'), 'SECRET_TOK', 'SECRET_TOKEN_2', 'test2@windmill.dev', 'user2 token', false)
ON CONFLICT DO NOTHING;
