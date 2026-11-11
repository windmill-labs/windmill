-- Fold edits made under ms_teams back into `teams`, which the previous version reads.
UPDATE global_settings
SET value = (value - 'ms_teams') || jsonb_build_object('teams', value -> 'ms_teams')
WHERE name = 'oauths' AND value ? 'ms_teams';
