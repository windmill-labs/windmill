-- Add parent_workspace_id and created_by columns to workspace table
-- This replaces the ephemeral_workspace table approach

-- Add parent_workspace_id column (optional, references workspace.id)
ALTER TABLE workspace 
ADD COLUMN parent_workspace_id character varying(50) REFERENCES workspace(id) ON DELETE SET NULL;

-- Add created_by column to store the username of the user who created the fork  
ALTER TABLE workspace 
ADD COLUMN created_by character varying(50);

-- Add created_at column to track when workspace was created
ALTER TABLE workspace 
ADD COLUMN created_at timestamp with time zone NOT NULL DEFAULT now();

-- Create index for performance on parent_workspace_id lookups
CREATE INDEX workspace_parent_idx ON workspace(parent_workspace_id) WHERE parent_workspace_id IS NOT NULL;

-- Migrate existing ephemeral workspace data from ephemeral_workspace table
UPDATE workspace 
SET parent_workspace_id = ew.parent_workspace_id
FROM ephemeral_workspace ew 
WHERE workspace.id = ew.ephemeral_workspace_id;

-- Grant appropriate permissions
GRANT SELECT, UPDATE ON workspace TO windmill_user;
GRANT SELECT, UPDATE ON workspace TO windmill_admin;