-- Add up migration script here
INSERT INTO config (name, config) VALUES  
    ('worker__default', '{"worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun"]}'), 
    ('worker__native', '{"worker_tags": ["nativets", "postgresql", "mysql", "graphql", "snowflake"]}') ON CONFLICT DO NOTHING;