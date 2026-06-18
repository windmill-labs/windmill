-- Add parent_workspace_id and created_by columns to workspace table

-- Add parent_workspace_id column (optional, references workspace.id)
ALTER TABLE workspace 
ADD COLUMN parent_workspace_id character varying(50) REFERENCES workspace(id) ON DELETE SET NULL;

-- Create index for performance on parent_workspace_id lookups
CREATE INDEX workspace_parent_idx ON workspace(parent_workspace_id) WHERE parent_workspace_id IS NOT NULL;
