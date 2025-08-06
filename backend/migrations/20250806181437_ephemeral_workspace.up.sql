-- Create ephemeral_workspace table to track ephemeral workspaces and their parents
CREATE TABLE ephemeral_workspace (
    ephemeral_workspace_id character varying(50) NOT NULL PRIMARY KEY REFERENCES workspace(id) ON DELETE CASCADE,
    parent_workspace_id character varying(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    created_at timestamp with time zone NOT NULL DEFAULT now()
);

-- Create index for performance on parent_workspace_id lookups
CREATE INDEX ephemeral_workspace_parent_idx ON ephemeral_workspace(parent_workspace_id);
