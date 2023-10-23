-- Add down migration script here
DELETE FROM global_settings WHERE name = 'uid';