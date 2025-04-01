ALTER TABLE workspace_settings
ADD COLUMN IF NOT EXISTS git_app_installations JSONB NOT NULL DEFAULT '[]';