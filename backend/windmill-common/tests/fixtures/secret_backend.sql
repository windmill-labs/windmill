-- Fixture for secret backend migration tests
-- Sets up test secrets in the variable table

-- Create a second workspace for testing workspace isolation
INSERT INTO workspace (id, name, owner)
VALUES ('test-workspace-2', 'test-workspace-2', 'test-user')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_settings (workspace_id)
VALUES ('test-workspace-2')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_key(workspace_id, kind, key)
VALUES ('test-workspace-2', 'cloud', 'test-key-2')
ON CONFLICT DO NOTHING;

-- Insert test variables with placeholder values.
-- Secret values are encrypted by the test setup using build_crypt + encrypt
-- with the workspace key, matching production behavior.
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES
    ('test-workspace', 'u/test-user/db_password', 'PLACEHOLDER', true, 'Database password', '{}'),
    ('test-workspace', 'u/test-user/api_key', 'PLACEHOLDER', true, 'API key for external service', '{}'),
    ('test-workspace', 'u/test-user/public_var', 'not-a-secret', false, 'A non-secret variable', '{}')
ON CONFLICT DO NOTHING;

-- Insert test secrets for workspace 2 (to test isolation)
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES
    ('test-workspace-2', 'u/test-user/other_secret', 'PLACEHOLDER', true, 'Secret in workspace 2', '{}')
ON CONFLICT DO NOTHING;
