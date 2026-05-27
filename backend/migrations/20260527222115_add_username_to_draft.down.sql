-- Add down migration script here
DROP INDEX IF EXISTS draft_pkey_legacy;
DROP INDEX IF EXISTS draft_pkey_with_user;

DELETE FROM draft WHERE username IS NOT NULL;

ALTER TABLE draft ADD CONSTRAINT draft_pkey PRIMARY KEY (workspace_id, path, typ);

ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_usr_fkey;

ALTER TABLE draft DROP COLUMN username;
