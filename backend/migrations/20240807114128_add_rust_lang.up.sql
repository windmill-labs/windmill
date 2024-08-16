-- Add up migration script here
ALTER TYPE SCRIPT_LANG ADD VALUE IF NOT EXISTS 'rust';
UPDATE config set config = '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "php", "rust"]}'::jsonb where name = 'worker__default' and config @> '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "php"]}'::jsonb;
