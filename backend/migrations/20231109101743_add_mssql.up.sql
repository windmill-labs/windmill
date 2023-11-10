-- Add up migration script here
ALTER TYPE SCRIPT_LANG ADD VALUE IF NOT EXISTS 'mssql';
-- Add up migration script here
UPDATE config set config = '{"worker_tags": ["nativets", "postgresql", "mysql", "graphql", "snowflake", "bigquery", "mssql"]}'::jsonb where name = 'worker__native' and config = '{"worker_tags": ["nativets", "postgresql", "mysql", "graphql", "snowflake", "bigquery"]}'::jsonb ;