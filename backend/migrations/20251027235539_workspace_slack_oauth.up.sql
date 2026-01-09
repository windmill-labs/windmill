-- Add workspace-level Slack OAuth configuration fields
ALTER TABLE workspace_settings
  ADD COLUMN slack_oauth_client_id VARCHAR(255) DEFAULT NULL,
  ADD COLUMN slack_oauth_client_secret VARCHAR(255) DEFAULT NULL;
