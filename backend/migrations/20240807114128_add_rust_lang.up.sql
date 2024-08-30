-- Add up migration script here
ALTER TYPE SCRIPT_LANG ADD VALUE IF NOT EXISTS 'rust';
UPDATE config set config = jsonb_set(config, '{worker_tags}', config->'worker_tags' || '["rust"]'::jsonb) where name = 'worker__default' and config @> '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "php"]}'::jsonb AND NOT config->'worker_tags' @> '"rust"'::jsonb;
