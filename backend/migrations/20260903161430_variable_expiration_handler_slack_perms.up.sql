-- Windmill never created this group before this release, so one present now was made by a user,
-- who would otherwise keep it as it gains read on the Slack bot token below.
DELETE FROM usr_to_group WHERE group_ = 'variable_expiration_handler';
UPDATE group_ SET extra_perms = '{}'::jsonb WHERE name = 'variable_expiration_handler';

-- Backfill for slack_bot folders created before this grant (see create_slack_workspace_artifacts).
UPDATE folder SET extra_perms = JSONB_SET(extra_perms, '{g/variable_expiration_handler}', 'false', true) WHERE name = 'slack_bot';
