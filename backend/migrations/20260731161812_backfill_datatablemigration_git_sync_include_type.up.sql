-- Data table SQL migrations became a git-sync object type (`datatablemigration`).
-- Existing settings were written before it existed, so every workspace would read
-- as "migrations opted out" until someone re-saved the form. Opt in the configs
-- that already sync something, at both levels an include_type can live: the
-- workspace-level default and each repository's own settings.
--
-- An absent or empty include_type means "inherit / not configured yet" and is left
-- alone; the defaults the UI and CLI now write already carry the new type.

UPDATE workspace_settings
SET git_sync = jsonb_set(
        git_sync,
        '{include_type}',
        (git_sync->'include_type') || '"datatablemigration"'::jsonb
    )
WHERE jsonb_typeof(git_sync->'include_type') = 'array'
  AND jsonb_array_length(git_sync->'include_type') > 0
  AND NOT (git_sync->'include_type' @> '"datatablemigration"'::jsonb);

UPDATE workspace_settings ws
SET git_sync = jsonb_set(ws.git_sync, '{repositories}', updated.repositories)
FROM (
    SELECT
        s.workspace_id,
        jsonb_agg(
            CASE
                WHEN jsonb_typeof(repo->'settings'->'include_type') = 'array'
                     AND jsonb_array_length(repo->'settings'->'include_type') > 0
                     AND NOT (repo->'settings'->'include_type' @> '"datatablemigration"'::jsonb)
                THEN jsonb_set(
                        repo,
                        '{settings,include_type}',
                        (repo->'settings'->'include_type') || '"datatablemigration"'::jsonb
                     )
                ELSE repo
            END
            ORDER BY idx
        ) AS repositories
    FROM workspace_settings s,
         LATERAL jsonb_array_elements(s.git_sync->'repositories') WITH ORDINALITY AS t(repo, idx)
    WHERE jsonb_typeof(s.git_sync->'repositories') = 'array'
    GROUP BY s.workspace_id
) AS updated
WHERE ws.workspace_id = updated.workspace_id
  AND ws.git_sync->'repositories' IS DISTINCT FROM updated.repositories;
