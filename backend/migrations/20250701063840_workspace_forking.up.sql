-- Add workspace forking tables

-- Table to track fork relationships between workspaces
CREATE TABLE workspace_fork (
    fork_workspace_id VARCHAR(63) PRIMARY KEY,
    parent_workspace_id VARCHAR(63) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    fork_point TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_fork_workspace FOREIGN KEY (fork_workspace_id) REFERENCES workspace(id) ON DELETE CASCADE,
    CONSTRAINT fk_parent_workspace FOREIGN KEY (parent_workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

-- Table to track which resources are references vs clones in forked workspaces
CREATE TABLE forked_resource_refs (
    id BIGSERIAL PRIMARY KEY,
    fork_workspace_id VARCHAR(63) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_path VARCHAR(4000) NOT NULL,
    is_reference BOOLEAN NOT NULL DEFAULT TRUE,
    parent_resource_id VARCHAR(4000),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_forked_resource_workspace FOREIGN KEY (fork_workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX idx_workspace_fork_parent ON workspace_fork(parent_workspace_id);
CREATE INDEX idx_forked_resource_refs_workspace ON forked_resource_refs(fork_workspace_id);
CREATE INDEX idx_forked_resource_refs_type_path ON forked_resource_refs(fork_workspace_id, resource_type, resource_path);
CREATE INDEX idx_forked_resource_refs_is_reference ON forked_resource_refs(fork_workspace_id, is_reference);

-- Grant permissions to windmill users
GRANT ALL ON workspace_fork TO windmill_user;
GRANT ALL ON workspace_fork TO windmill_admin;
GRANT ALL ON forked_resource_refs TO windmill_user;
GRANT ALL ON forked_resource_refs TO windmill_admin;
GRANT ALL ON SEQUENCE forked_resource_refs_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE forked_resource_refs_id_seq TO windmill_admin;