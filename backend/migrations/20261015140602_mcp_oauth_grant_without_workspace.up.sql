-- An MCP OAuth grant can span several workspaces (listed in its scopes) or all of the
-- user's, in which case the token it mints is bound to none.
ALTER TABLE mcp_oauth_server_code ALTER COLUMN workspace_id DROP NOT NULL;
ALTER TABLE mcp_oauth_refresh_token ALTER COLUMN workspace_id DROP NOT NULL;
