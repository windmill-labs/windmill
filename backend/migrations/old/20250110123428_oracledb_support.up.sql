-- Add up migration script here
ALTER TYPE SCRIPT_LANG ADD VALUE IF NOT EXISTS 'oracledb';
UPDATE config set config = jsonb_set(config, '{worker_tags}', config->'worker_tags' || '["oracledb"]'::jsonb) where name = 'worker__native' and config @> '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "php", "rust", "ansible", "csharp"]}'::jsonb AND NOT config->'worker_tags' @> '"oracledb"'::jsonb;
