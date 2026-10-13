-- Backfill for slack_bot folders created before this grant (see create_slack_workspace_artifacts).
UPDATE folder SET extra_perms = JSONB_SET(extra_perms, '{g/variable_expiration_handler}', 'false', true) WHERE name = 'slack_bot';
