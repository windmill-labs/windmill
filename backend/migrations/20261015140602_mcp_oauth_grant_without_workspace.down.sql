-- A server without these scopes ignores them, which would widen every token carrying one:
-- a narrowed `mcp:all` to full access, a workspace list to all of the owner's workspaces.
DELETE FROM token WHERE EXISTS (
    SELECT 1 FROM unnest(scopes) s
    WHERE s IN ('mcp:read_only', 'mcp:read_run', 'mcp:no_endpoints') OR s LIKE 'mcp:workspaces:%'
);
DELETE FROM mcp_oauth_refresh_token WHERE EXISTS (
    SELECT 1 FROM unnest(scopes) s
    WHERE s IN ('mcp:read_only', 'mcp:read_run', 'mcp:no_endpoints') OR s LIKE 'mcp:workspaces:%'
);
DELETE FROM mcp_oauth_server_code WHERE EXISTS (
    SELECT 1 FROM unnest(scopes) s
    WHERE s IN ('mcp:read_only', 'mcp:read_run', 'mcp:no_endpoints') OR s LIKE 'mcp:workspaces:%'
);
DELETE FROM mcp_oauth_server_code WHERE workspace_id IS NULL;
DELETE FROM mcp_oauth_refresh_token WHERE workspace_id IS NULL;
ALTER TABLE mcp_oauth_server_code ALTER COLUMN workspace_id SET NOT NULL;
ALTER TABLE mcp_oauth_refresh_token ALTER COLUMN workspace_id SET NOT NULL;
