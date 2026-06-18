-- Re-add the column to resource
ALTER TABLE resource ADD COLUMN IF NOT EXISTS ws_specific BOOLEAN NOT NULL DEFAULT false;

-- Migrate data back
UPDATE resource SET ws_specific = true
FROM ws_specific ws
WHERE ws.workspace_id = resource.workspace_id
  AND ws.item_kind = 'resource'
  AND ws.path = resource.path;

-- Drop the table
DROP TABLE IF EXISTS ws_specific;
