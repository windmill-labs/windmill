-- The workspace's own list of Hub integrations members without code may use,
-- `{"<integration>": "read" | "write"}`. NULL follows the instance's `hub_access` setting.
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS hub_access JSONB;
