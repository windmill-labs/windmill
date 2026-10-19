-- The Teams Entra app now lives under oauths.ms_teams (bot + Graph connect). Copy the old
-- `teams` entry and keep it, so servers still on the previous version keep working during a
-- rolling upgrade.
UPDATE global_settings
SET value = value || jsonb_build_object('ms_teams', value -> 'teams')
WHERE name = 'oauths' AND value ? 'teams' AND NOT value ? 'ms_teams';
