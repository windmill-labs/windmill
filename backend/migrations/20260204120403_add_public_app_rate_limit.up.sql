ALTER TABLE workspace_settings
ADD COLUMN IF NOT EXISTS public_app_execution_limit_per_minute INTEGER DEFAULT NULL;
