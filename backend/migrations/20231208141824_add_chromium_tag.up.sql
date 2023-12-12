-- Add up migration script here
INSERT INTO global_settings VALUES ('custom_tags', '["chromium"]')
ON CONFLICT (name) DO UPDATE SET value = jsonb_insert(global_settings.value, '{0}', '"chromium"', true)
WHERE NOT global_settings.value @> '["chromium"]';


