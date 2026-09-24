-- Grant the variable expiration handler group read access to the Slack bot token, so the
-- built-in Slack handler can post. Mirrors 20231016142659 for g/error_handler: the folder's
-- extra_perms is written once at Slack connect (ON CONFLICT DO NOTHING) and never refreshed,
-- so workspaces that connected Slack before this feature need the backfill to get the grant.
UPDATE folder SET extra_perms = JSONB_SET(extra_perms, '{g/variable_expiration_handler}', 'false', true) WHERE name = 'slack_bot';
