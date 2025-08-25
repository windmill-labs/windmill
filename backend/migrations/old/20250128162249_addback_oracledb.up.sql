-- Add up migration script here
UPDATE config set config = jsonb_set(config, '{worker_tags}', config->'worker_tags' || '["oracledb"]'::jsonb) where name = 'worker__native' and config @> '{"worker_tags": ["nativets", "postgresql", "mysql", "graphql", "snowflake", "bigquery", "mssql"]}'::jsonb AND NOT config->'worker_tags' @> '"oracledb"'::jsonb;
