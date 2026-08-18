-- Backfill the owner `email` on legacy drafts (rows persisted before per-user
-- sync, hence `email IS NULL`). A user-owned draft path is `u/<username>/...`,
-- so resolve `<username>` against `usr` for the same workspace and adopt that
-- user's email.
--
-- Guards:
--   - the resolved email must exist in `password` (the `draft_password_fkey`
--     target), or the UPDATE would violate the FK;
--   - skip rows that would collide with an existing per-user draft at the same
--     (workspace_id, path, typ, email) under the `draft_pkey_with_user` partial
--     unique index — the per-user row is authoritative, so the legacy row is
--     left untouched.
UPDATE draft d
SET email = u.email
FROM usr u
WHERE d.email IS NULL
  AND split_part(d.path, '/', 1) = 'u'
  AND split_part(d.path, '/', 2) <> ''
  AND u.workspace_id = d.workspace_id
  AND u.username = split_part(d.path, '/', 2)
  AND EXISTS (SELECT 1 FROM password p WHERE p.email = u.email)
  AND NOT EXISTS (
    SELECT 1 FROM draft d2
    WHERE d2.workspace_id = d.workspace_id
      AND d2.path = d.path
      AND d2.typ = d.typ
      AND d2.email = u.email
  );
