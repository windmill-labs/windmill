DELETE FROM mcp_oauth_server_code WHERE workspace_id IS NULL;
DELETE FROM mcp_oauth_refresh_token WHERE workspace_id IS NULL;
ALTER TABLE mcp_oauth_server_code ALTER COLUMN workspace_id SET NOT NULL;
ALTER TABLE mcp_oauth_refresh_token ALTER COLUMN workspace_id SET NOT NULL;
