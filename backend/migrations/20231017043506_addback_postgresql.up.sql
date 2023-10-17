-- Add up migration script here
UPDATE config set   config = '{"worker_tags": ["nativets", "postgresql", "mysql", "graphql", "snowflake", "bigquery"]}'::jsonb  
where name = 'worker__native' and  config = '{"worker_tags": ["nativets", "postgresq", "mysql", "graphql", "snowflake", "bigquery"]}'::jsonb;