-- Remove workspace-level Slack OAuth configuration fields
ALTER TABLE workspace_settings
  DROP COLUMN IF EXISTS slack_oauth_client_secret,
  DROP COLUMN IF EXISTS slack_oauth_client_id;
