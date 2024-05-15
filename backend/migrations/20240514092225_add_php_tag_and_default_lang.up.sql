-- Add up migration script here
UPDATE config set config = '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "php"]}'::jsonb where name = 'worker__default' and config @> '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun"]}'::jsonb;
UPDATE workspace_settings SET default_scripts = jsonb_set(default_scripts, '{order}', default_scripts->'order' || '["php"]'::jsonb) WHERE default_scripts IS NOT NULL AND default_scripts->'order' IS NOT NULL AND NOT default_scripts->'order' @> '["php"]'::jsonb;

