ALTER TABLE draft DROP COLUMN id;

-- The secret-draft scrub in the .up.sql is intentionally irreversible —
-- the plaintext values it blanked were deliberately destroyed and cannot
-- be recovered here.

DROP INDEX IF EXISTS draft_workspace_path_typ_idx;
DROP INDEX IF EXISTS draft_user_sync_idx;
DROP INDEX IF EXISTS draft_pkey_legacy;
DROP INDEX IF EXISTS draft_pkey_with_user;

-- Per-user rows can't be represented in the pre-sync schema (one row per
-- (workspace_id, path, typ)). Drop them before restoring the composite PK.
DELETE FROM draft WHERE email IS NOT NULL;

ALTER TABLE draft ADD CONSTRAINT draft_pkey PRIMARY KEY (workspace_id, path, typ);

ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_password_fkey;
ALTER TABLE draft DROP COLUMN email;

-- Restore the narrower DRAFT_TYPE enum (script/flow/app only). Drop any
-- rows whose kind falls outside that set so the cast doesn't fail.
CREATE TYPE DRAFT_TYPE AS ENUM ('script', 'flow', 'app');
DELETE FROM draft WHERE typ::text NOT IN ('script', 'flow', 'app');
ALTER TABLE draft ALTER COLUMN typ TYPE DRAFT_TYPE USING typ::text::DRAFT_TYPE;
DROP TYPE DRAFT_KIND;
