-- Add up migration script here
UPDATE password SET username = NULL WHERE email = 'admin@windmill.dev'
AND (SELECT value FROM global_settings WHERE name = 'automate_username_creation') IS DISTINCT FROM 'true';