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

-- Insert test secrets for workspace 1
-- Note: The 'value' column stores encrypted values in production,
-- but for tests we'll use plain text that the migration will handle
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES
    ('test-workspace', 'u/test-user/db_password', 'encrypted-db-pass-123', true, 'Database password', '{}'),
    ('test-workspace', 'u/test-user/api_key', 'encrypted-api-key-abc', true, 'API key for external service', '{}'),
    ('test-workspace', 'u/test-user/public_var', 'not-a-secret', false, 'A non-secret variable', '{}')
ON CONFLICT DO NOTHING;

-- Insert test secrets for workspace 2 (to test isolation)
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES
    ('test-workspace-2', 'u/test-user/other_secret', 'encrypted-other-secret', true, 'Secret in workspace 2', '{}')
ON CONFLICT DO NOTHING;
