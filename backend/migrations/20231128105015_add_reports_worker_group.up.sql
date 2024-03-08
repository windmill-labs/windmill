-- Add up migration script here
INSERT INTO config (name, config) VALUES  
    ('worker__reports', '{"init_bash": "apt-get update\napt-get install -y chromium", "worker_tags": ["deno", "python3", "go", "bash", "powershell", "dependency", "flow", "hub", "other", "bun", "chromium"]}') ON CONFLICT DO NOTHING;