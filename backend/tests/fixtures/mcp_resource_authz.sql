-- Fixture for the MCP resource-authorization regression test (WIN-2041,
-- GHSA-7qg3-pr4g-cq5x).
--
-- Models a low-privileged developer (test-user-3, a plain workspace member) who
-- references, from an AI Agent flow, an MCP resource living in a private folder
-- they have NO access to. The AI agent worker must refuse to load that resource
-- because the developer's identity lacks resources:read on it.

-- Private folder the developer cannot read (empty extra_perms, owned by admin).
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'private', 'Private Folder', '{"u/test-user"}', '{}', 'test-user');

-- MCP resource the developer is NOT allowed to read. The URL is a non-resolvable
-- public host so that, for an authorized caller, resolution succeeds but the
-- later connection step fails deterministically without network access.
INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES (
    'test-workspace',
    'f/private/admin_mcp',
    '{"name": "admin_mcp", "url": "https://mcp.invalid.windmill.test"}',
    'Private MCP resource only admin can read',
    'mcp',
    '{}',
    'test-user'
);

-- MCP resource whose URL lives in a variable. The host only appears in the
-- variable, so an error naming it proves the reference was resolved.
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/mcp_url', 'https://mcp-from-var.invalid.windmill.test/key', false, '', '{}');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES (
    'test-workspace',
    'u/test-user/var_mcp',
    '{"name": "var_mcp", "url": "$var:u/test-user/mcp_url"}',
    'MCP resource with its URL in a variable',
    'mcp',
    '{}',
    'test-user'
);
