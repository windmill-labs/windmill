-- Add up migration script here
INSERT INTO global_settings (name, value, updated_at) VALUES ('uid', to_jsonb(gen_random_uuid()), now()) ON CONFLICT DO NOTHING;