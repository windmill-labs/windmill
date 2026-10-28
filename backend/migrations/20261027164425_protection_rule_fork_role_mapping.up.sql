ALTER TABLE workspace_protection_rule
    ADD COLUMN IF NOT EXISTS fork_role_mapping JSONB NOT NULL DEFAULT '{}'::jsonb;
