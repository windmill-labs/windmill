-- Server-owned git-sync credentials, one entry per repository the workspace
-- holds a token for, keyed by that repository rather than by a resource naming
-- it: a resource's URL is writable, and its path is not settled while it is
-- being created.
-- Kept out of `git_sync` because that column is copied into forks and returned
-- by the workspace settings API; this one is copied by neither.
ALTER TABLE workspace_settings
    ADD COLUMN IF NOT EXISTS git_credentials JSONB NOT NULL DEFAULT '[]'::jsonb;
