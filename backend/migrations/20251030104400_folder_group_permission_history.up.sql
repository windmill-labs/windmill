-- Add up migration script here

-- Folder permission changes history
CREATE TABLE IF NOT EXISTS folder_permission_history (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    folder_name VARCHAR(255) NOT NULL,
    changed_by VARCHAR(50) NOT NULL,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    change_type VARCHAR(50) NOT NULL,
    affected VARCHAR(100),
    FOREIGN KEY (workspace_id, folder_name) REFERENCES folder(workspace_id, name) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_folder_perm_history_workspace_folder
    ON folder_permission_history(workspace_id, folder_name, id DESC);

-- Group permission changes history
CREATE TABLE IF NOT EXISTS group_permission_history (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    group_name VARCHAR(255) NOT NULL,
    changed_by VARCHAR(50) NOT NULL,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    change_type VARCHAR(50) NOT NULL,
    member_affected VARCHAR(100),
    FOREIGN KEY (workspace_id, group_name) REFERENCES group_(workspace_id, name) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_group_perm_history_workspace_group
    ON group_permission_history(workspace_id, group_name, id DESC);
