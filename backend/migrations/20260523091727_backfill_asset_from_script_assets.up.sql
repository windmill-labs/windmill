-- Backfill the `asset` table from `script.assets` JSONB for scripts that
-- were deployed before the deploy path started populating asset rows from
-- the parsed `ns.assets`. The JSONB column carried the parser's findings
-- the whole time; the corresponding asset rows are what `fetch_producer_writes`
-- (asset-trigger cascade) and the asset-graph lineage view actually query.
-- Without this, a pre-feature script "succeeds" but the dispatcher sees
-- no writes, no subscribers are matched, and the dispatch_event panel
-- stays empty.
--
-- Idempotent: scoped to (workspace, path) pairs that have ZERO asset rows
-- under `usage_kind = 'script'`, so re-running can't duplicate. The
-- (workspace_id, path, kind, usage_path, usage_kind) primary key catches
-- any residual overlap via `ON CONFLICT DO NOTHING` as a belt-and-braces.
-- Skips archived/deleted script versions so we backfill from the *current*
-- snapshot, matching what the live deploy path would write.
INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
SELECT
  s.workspace_id,
  a->>'path',
  (a->>'kind')::asset_kind,
  (a->>'access_type')::asset_access_type,
  s.path,
  'script'::asset_usage_kind
FROM script s
CROSS JOIN LATERAL jsonb_array_elements(s.assets) AS a
WHERE s.archived = false
  AND s.deleted = false
  AND s.assets IS NOT NULL
  AND jsonb_typeof(s.assets) = 'array'
  AND a->>'path' IS NOT NULL
  AND a->>'kind' IS NOT NULL
  AND NOT EXISTS (
    SELECT 1
    FROM asset existing
    WHERE existing.workspace_id = s.workspace_id
      AND existing.usage_kind = 'script'
      AND existing.usage_path = s.path
  )
ON CONFLICT DO NOTHING;
