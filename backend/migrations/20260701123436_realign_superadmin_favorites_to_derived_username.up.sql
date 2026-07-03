-- A superadmin acting in a workspace they are not a member of used to be
-- identified by their raw email (so favorites were stored with usr = email).
-- They are now identified by their instance-derived username (password.username),
-- so realign those pre-existing favorites to keep them visible. Only email-keyed
-- rows are ever a superadmin's (members always store a non-email username), and
-- the anti-join skips rows that would collide with an already-derived favorite.
UPDATE favorite f
SET usr = p.username
FROM password p
WHERE f.usr = p.email
  AND p.super_admin = true
  AND p.username IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM favorite f2
    WHERE f2.workspace_id = f.workspace_id
      AND f2.usr = p.username
      AND f2.favorite_kind = f.favorite_kind
      AND f2.path = f.path
  );
