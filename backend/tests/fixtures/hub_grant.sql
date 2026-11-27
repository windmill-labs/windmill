-- Workspace `hub-grant`, where `operator` composes flows and Slack's read-only Hub scripts are
-- allowed. Its own workspace: roles are cached per workspace id across a test binary's tests.
-- test-user is its admin, test-user-3 (SECRET_TOKEN_3) its operator.
INSERT INTO workspace (id, name, owner) VALUES ('hub-grant', 'hub-grant', 'test-user');
INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('hub-grant', 'cloud', 'test-key');
INSERT INTO workspace_settings (workspace_id, role_overrides, hub_access) VALUES ('hub-grant',
    '{"operator": {"capabilities": {"flows": "compose"}}}', '{"Slack": "read"}');
INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ('hub-grant', 'all', 'All users', '{}');
INSERT INTO usr (workspace_id, email, username, is_admin, operator, role) VALUES
    ('hub-grant', 'test@windmill.dev', 'test-user', true, false, 'Admin'),
    ('hub-grant', 'test3@windmill.dev', 'test-user-3', false, true, 'Operator');
