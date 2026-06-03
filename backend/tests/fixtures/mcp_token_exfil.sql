-- Fixture for the MCP token-exfiltration regression test.
--
-- Models a malicious developer (test-user-3, a plain workspace member) who:
--   - owns an MCP resource they are allowed to read, and
--   - points that resource's `token` field at a secret variable living in a
--     folder they have NO access to (`f/locked`, only test-user/admin owns it).
--
-- The secret variable `f/locked/secret_token` itself is inserted by the test in
-- Rust (so it is encrypted with the real workspace key); this fixture only sets
-- up the locked folder, the resource, and their permissions.

-- Folder the developer cannot read (empty extra_perms, owned by admin only).
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'locked', 'Locked Folder', '{"u/test-user"}', '{}', 'test-user');

-- MCP resource owned by the developer (so RLS lets them read the resource),
-- whose token references the locked secret. The URL is a non-resolvable public
-- host so that, for an authorized caller, resolution succeeds but the later
-- connection/SSRF step fails deterministically without network access.
INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES (
    'test-workspace',
    'u/test-user-3/evil_mcp',
    '{"name": "evil", "url": "https://mcp.invalid.windmill.test", "token": "$var:f/locked/secret_token"}',
    'MCP resource whose token points at a locked secret',
    'mcp',
    '{}',
    'test-user-3'
);
