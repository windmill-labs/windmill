-- Add up migration script here
UPDATE folder SET extra_perms = JSONB_SET(extra_perms, '{g/error_handler}', 'false', true) WHERE name = 'slack_bot';
