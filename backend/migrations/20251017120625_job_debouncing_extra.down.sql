-- Add down migration script here
DROP INDEX IF EXISTS idx_script_workspace_debounce_key;
ALTER TABLE script DROP COLUMN IF EXISTS debounce_key;
ALTER TABLE script DROP COLUMN IF EXISTS debounce_delay_s;

