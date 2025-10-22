-- Add up migration script here
ALTER TABLE script ADD COLUMN debounce_key VARCHAR(255);
ALTER TABLE script ADD COLUMN debounce_delay_s INTEGER;

-- Add index on debounce_key for efficient lookups during debounce operations
CREATE INDEX IF NOT EXISTS idx_script_workspace_debounce_key ON script(workspace_id, debounce_key) WHERE debounce_key IS NOT NULL;
