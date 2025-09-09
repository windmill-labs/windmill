-- Add up migration script here
UPDATE capture SET trigger_kind = 'default_email' WHERE trigger_kind = 'email';
UPDATE capture_config SET trigger_kind = 'default_email' WHERE trigger_kind = 'email';