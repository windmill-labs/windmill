-- Add down migration script here
UPDATE capture SET trigger_kind = 'email' WHERE trigger_kind = 'default_email';
UPDATE capture_config SET trigger_kind = 'email' WHERE trigger_kind = 'default_email';