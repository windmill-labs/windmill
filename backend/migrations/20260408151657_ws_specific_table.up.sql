-- Create the ws_specific table
CREATE TABLE IF NOT EXISTS ws_specific (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
    item_kind VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    PRIMARY KEY (workspace_id, item_kind, path)
);

-- Migrate existing data from resource.ws_specific
INSERT INTO ws_specific (workspace_id, item_kind, path)
SELECT workspace_id, 'resource', path
FROM resource
WHERE ws_specific = true
ON CONFLICT DO NOTHING;

-- Drop the column from resource
ALTER TABLE resource DROP COLUMN IF EXISTS ws_specific;
