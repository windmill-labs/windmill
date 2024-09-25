-- Add up migration script here
INSERT INTO global_settings (name, value)
SELECT 'smtp_settings', config
FROM config
WHERE name = 'server'
ON CONFLICT (name) DO UPDATE SET value = excluded.value;

DELETE FROM config WHERE name = 'server';
