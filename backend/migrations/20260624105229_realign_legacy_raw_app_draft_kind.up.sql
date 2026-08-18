-- The pre-per-user `DRAFT_TYPE` enum had only ('script','flow','app'): a raw
-- app's draft was therefore stored as typ='app'. The new model splits app vs
-- raw_app into distinct draft kinds chosen from the deployed app's `raw_app`
-- flag, so a raw app's pre-migration draft is invisible to the per-kind lookups
-- (editor overlay, migrate-legacy, get-for-user), which all query typ='raw_app'.
-- Realign every such draft (any owner, including the legacy NULL-email row) to
-- 'raw_app' when the deployed app at that path is a raw app.

-- Drop, don't retype, a stale 'app' row when a 'raw_app' draft already exists
-- for the same owner (the newer 'raw_app' row, saved with the per-kind code, is
-- authoritative) — retyping would collide on the draft_pkey_with_user /
-- draft_pkey_legacy partial unique indexes over (workspace_id, path, typ, email).
DELETE FROM draft d
USING app a
JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
WHERE d.typ = 'app'
  AND a.workspace_id = d.workspace_id
  AND a.path = d.path
  AND av.raw_app IS TRUE
  AND EXISTS (
    SELECT 1 FROM draft d2
    WHERE d2.workspace_id = d.workspace_id
      AND d2.path = d.path
      AND d2.typ = 'raw_app'
      AND d2.email IS NOT DISTINCT FROM d.email
  );

UPDATE draft d
SET typ = 'raw_app'
FROM app a
JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
WHERE d.typ = 'app'
  AND a.workspace_id = d.workspace_id
  AND a.path = d.path
  AND av.raw_app IS TRUE;
