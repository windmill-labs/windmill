-- Strip `datatablemigration` back out of every include_type, at both levels.
-- Lossy in the same way the up is: a config that had opted in deliberately is
-- indistinguishable from one the backfill touched.

UPDATE workspace_settings
SET git_sync = jsonb_set(
        git_sync,
        '{include_type}',
        (git_sync->'include_type') - 'datatablemigration'
    )
WHERE jsonb_typeof(git_sync->'include_type') = 'array'
  AND git_sync->'include_type' @> '"datatablemigration"'::jsonb;

UPDATE workspace_settings ws
SET git_sync = jsonb_set(ws.git_sync, '{repositories}', updated.repositories)
FROM (
    SELECT
        s.workspace_id,
        jsonb_agg(
            CASE
                WHEN repo->'settings'->'include_type' @> '"datatablemigration"'::jsonb
                THEN jsonb_set(
                        repo,
                        '{settings,include_type}',
                        (repo->'settings'->'include_type') - 'datatablemigration'
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
