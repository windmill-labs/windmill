-- Drafts become user-scoped: each draft now records the email of the user that
-- owns it. The column is nullable so that legacy drafts (and rows transferred
-- out of `draft_only` in a later migration) are preserved with a NULL owner
-- rather than being lost.
ALTER TABLE draft ADD COLUMN email VARCHAR(255);

-- The old primary key (workspace_id, path, typ) allowed a single draft per
-- path. Now that drafts are per-user we need at most one draft per
-- (workspace_id, path, typ, owner). We use COALESCE(email, '') so the NULL
-- (legacy) bucket collapses to a single key as well — this keeps the
-- constraint working on every supported Postgres version (unlike
-- `UNIQUE NULLS NOT DISTINCT`, which requires PG15+).
ALTER TABLE draft DROP CONSTRAINT draft_pkey;
CREATE UNIQUE INDEX draft_workspace_id_path_typ_email_key
    ON draft (workspace_id, path, typ, COALESCE(email, ''));
