-- Add workspace_protection_rule table for fine-grained access control
CREATE TABLE workspace_protection_rule (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    rules INTEGER NOT NULL,
    bypass_groups TEXT[] NOT NULL DEFAULT '{}',
    bypass_users TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, name)
);

-- Index for listing all rules in a workspace
CREATE INDEX idx_protection_rule_workspace ON workspace_protection_rule(workspace_id);

-- GIN indexes for efficient containment checks (checking if user/group is in bypass list)
CREATE INDEX idx_protection_rule_bypass_groups ON workspace_protection_rule USING GIN (bypass_groups);
CREATE INDEX idx_protection_rule_bypass_users ON workspace_protection_rule USING GIN (bypass_users);
