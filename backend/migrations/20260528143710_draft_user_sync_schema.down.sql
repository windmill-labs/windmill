ALTER TABLE draft DROP COLUMN id;

-- The .up.sql secret-draft scrub is irreversible — the blanked plaintext
-- values were deliberately destroyed and cannot be recovered here.

DROP INDEX IF EXISTS draft_workspace_path_typ_idx;
DROP INDEX IF EXISTS draft_user_listing_idx;
DROP INDEX IF EXISTS draft_pkey_legacy;
DROP INDEX IF EXISTS draft_pkey_with_user;

-- Per-user rows can't exist under the composite PK (one row per
-- (workspace_id, path, typ)); drop them before restoring it.
DELETE FROM draft WHERE email IS NOT NULL;

ALTER TABLE draft ADD CONSTRAINT draft_pkey PRIMARY KEY (workspace_id, path, typ);

ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_password_fkey;
ALTER TABLE draft DROP COLUMN email;

-- Restore the narrower DRAFT_TYPE enum; drop rows outside that set so the
-- cast doesn't fail.
CREATE TYPE DRAFT_TYPE AS ENUM ('script', 'flow', 'app');
DELETE FROM draft WHERE typ::text NOT IN ('script', 'flow', 'app');
ALTER TABLE draft ALTER COLUMN typ TYPE DRAFT_TYPE USING typ::text::DRAFT_TYPE;
DROP TYPE DRAFT_KIND;
