-- Add added_via column to track how users were added to workspaces
-- NULL = manual addition
-- {"source": "domain", "domain": "company.com"} = domain auto-add
-- {"source": "instance_group", "group": "developers", "role": "developer"} = instance group auto-add
ALTER TABLE usr ADD COLUMN added_via jsonb DEFAULT NULL;

-- Add index for efficient queries on added_via
CREATE INDEX idx_usr_added_via ON usr USING gin (added_via);
