-- Server-owned git-sync credentials, one entry per repo resource path.
-- Kept out of `git_sync` because that column is copied into forks and returned
-- by the workspace settings API; this one is copied by neither.
ALTER TABLE workspace_settings
    ADD COLUMN IF NOT EXISTS git_credentials JSONB NOT NULL DEFAULT '[]'::jsonb;
