-- Add workspace merge request and conflict detection tables

-- Table to track merge requests between workspaces
CREATE TABLE workspace_merge_request (
    id BIGSERIAL PRIMARY KEY,
    source_workspace_id VARCHAR(63) NOT NULL,
    target_workspace_id VARCHAR(63) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'approved', 'rejected', 'merged', 'conflicted'
    title VARCHAR(500) NOT NULL,
    description TEXT,
    merged_at TIMESTAMPTZ,
    merged_by VARCHAR(255),
    rejected_at TIMESTAMPTZ,
    rejected_by VARCHAR(255),
    rejection_reason TEXT,
    auto_merge BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT fk_merge_source_workspace FOREIGN KEY (source_workspace_id) REFERENCES workspace(id) ON DELETE CASCADE,
    CONSTRAINT fk_merge_target_workspace FOREIGN KEY (target_workspace_id) REFERENCES workspace(id) ON DELETE CASCADE,
    CONSTRAINT chk_merge_status CHECK (status IN ('pending', 'approved', 'rejected', 'merged', 'conflicted'))
);

-- Table to track changes in a merge request
CREATE TABLE workspace_merge_change (
    id BIGSERIAL PRIMARY KEY,
    merge_request_id BIGINT NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_path VARCHAR(4000) NOT NULL,
    change_type VARCHAR(20) NOT NULL, -- 'added', 'modified', 'deleted'
    source_content_hash VARCHAR(64), -- SHA256 hash of source content
    target_content_hash VARCHAR(64), -- SHA256 hash of target content
    has_conflict BOOLEAN NOT NULL DEFAULT FALSE,
    conflict_reason TEXT,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolution_strategy VARCHAR(20), -- 'take_source', 'take_target', 'manual'
    resolved_by VARCHAR(255),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_merge_change_request FOREIGN KEY (merge_request_id) REFERENCES workspace_merge_request(id) ON DELETE CASCADE,
    CONSTRAINT chk_change_type CHECK (change_type IN ('added', 'modified', 'deleted')),
    CONSTRAINT chk_resolution_strategy CHECK (resolution_strategy IS NULL OR resolution_strategy IN ('take_source', 'take_target', 'manual'))
);

-- Table to store content snapshots for merge diff and conflict resolution
CREATE TABLE workspace_merge_content (
    id BIGSERIAL PRIMARY KEY,
    merge_change_id BIGINT NOT NULL,
    content_type VARCHAR(20) NOT NULL, -- 'source', 'target', 'base', 'resolved'
    content_hash VARCHAR(64) NOT NULL,
    content_data JSONB, -- Store the actual content for comparison
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_merge_content_change FOREIGN KEY (merge_change_id) REFERENCES workspace_merge_change(id) ON DELETE CASCADE,
    CONSTRAINT chk_content_type CHECK (content_type IN ('source', 'target', 'base', 'resolved'))
);

-- Table to track workspace modification timestamps for conflict detection
CREATE TABLE workspace_resource_timestamp (
    workspace_id VARCHAR(63) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_path VARCHAR(4000) NOT NULL,
    last_modified TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    modified_by VARCHAR(255) NOT NULL,
    content_hash VARCHAR(64),
    PRIMARY KEY (workspace_id, resource_type, resource_path),
    CONSTRAINT fk_resource_timestamp_workspace FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

-- Table to track merge permissions and approvers
CREATE TABLE workspace_merge_approver (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(63) NOT NULL,
    user_email VARCHAR(255) NOT NULL,
    can_approve_merges BOOLEAN NOT NULL DEFAULT TRUE,
    can_auto_merge BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT fk_merge_approver_workspace FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE,
    CONSTRAINT uq_merge_approver_user UNIQUE (workspace_id, user_email)
);

-- Indexes for efficient queries
CREATE INDEX idx_workspace_merge_request_source ON workspace_merge_request(source_workspace_id, status);
CREATE INDEX idx_workspace_merge_request_target ON workspace_merge_request(target_workspace_id, status);
CREATE INDEX idx_workspace_merge_request_created_at ON workspace_merge_request(created_at DESC);

CREATE INDEX idx_workspace_merge_change_request ON workspace_merge_change(merge_request_id);
CREATE INDEX idx_workspace_merge_change_conflict ON workspace_merge_change(merge_request_id, has_conflict);
CREATE INDEX idx_workspace_merge_change_resource ON workspace_merge_change(resource_type, resource_path);

CREATE INDEX idx_workspace_merge_content_change ON workspace_merge_content(merge_change_id, content_type);
CREATE INDEX idx_workspace_merge_content_hash ON workspace_merge_content(content_hash);

CREATE INDEX idx_workspace_resource_timestamp_modified ON workspace_resource_timestamp(workspace_id, last_modified DESC);
CREATE INDEX idx_workspace_resource_timestamp_hash ON workspace_resource_timestamp(content_hash);

CREATE INDEX idx_workspace_merge_approver_workspace ON workspace_merge_approver(workspace_id);

-- Grant permissions to windmill users
GRANT ALL ON workspace_merge_request TO windmill_user;
GRANT ALL ON workspace_merge_request TO windmill_admin;
GRANT ALL ON SEQUENCE workspace_merge_request_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE workspace_merge_request_id_seq TO windmill_admin;

GRANT ALL ON workspace_merge_change TO windmill_user;
GRANT ALL ON workspace_merge_change TO windmill_admin;
GRANT ALL ON SEQUENCE workspace_merge_change_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE workspace_merge_change_id_seq TO windmill_admin;

GRANT ALL ON workspace_merge_content TO windmill_user;
GRANT ALL ON workspace_merge_content TO windmill_admin;
GRANT ALL ON SEQUENCE workspace_merge_content_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE workspace_merge_content_id_seq TO windmill_admin;

GRANT ALL ON workspace_resource_timestamp TO windmill_user;
GRANT ALL ON workspace_resource_timestamp TO windmill_admin;

GRANT ALL ON workspace_merge_approver TO windmill_user;
GRANT ALL ON workspace_merge_approver TO windmill_admin;
GRANT ALL ON SEQUENCE workspace_merge_approver_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE workspace_merge_approver_id_seq TO windmill_admin;