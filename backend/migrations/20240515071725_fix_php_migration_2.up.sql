-- Add up migration script here
UPDATE config SET name = 'worker__default' WHERE name = 'worker__default_tmp';
UPDATE config set config = jsonb_set(config, '{worker_tags}', config->'worker_tags' || '["php"]'::jsonb) where name = 'worker__default' and config @> '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun"]}'::jsonb AND NOT config->'worker_tags' @> '"php"'::jsonb;
