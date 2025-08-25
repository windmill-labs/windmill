INSERT INTO global_settings (name, value) 
VALUES ('otel', '{}') 
ON CONFLICT (name) DO NOTHING;
