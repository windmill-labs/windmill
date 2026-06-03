DROP INDEX IF EXISTS draft_workspace_id_path_typ_email_key;

-- Restoring the old (workspace_id, path, typ) primary key requires uniqueness
-- on those three columns. Per-user drafts created while this migration was
-- applied may have introduced duplicates, so collapse them first (keep an
-- arbitrary surviving row per key) before re-adding the constraint.
DELETE FROM draft a
USING draft b
WHERE a.ctid < b.ctid
  AND a.workspace_id = b.workspace_id
  AND a.path = b.path
  AND a.typ = b.typ;

ALTER TABLE draft ADD PRIMARY KEY (workspace_id, path, typ);
ALTER TABLE draft DROP COLUMN email;
