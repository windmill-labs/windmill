-- Shape mirrors `error_handler` / `success_handler` as grouped by 20260124172000:
-- {"path": ..., "extra_args": ..., "muted_on_user_path": ...}
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS variable_expiration_handler JSONB;

-- When the handler last failed to start; the sweep skips the workspace for a while after it.
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS variable_expiration_failed_at TIMESTAMP WITH TIME ZONE;
