-- Add teams_team_guid column to store the GUID (used for MS Graph API calls)
-- The existing teams_team_id column stores the internal_id (used for webhook matching)
ALTER TABLE workspace_settings ADD COLUMN teams_team_guid TEXT;
