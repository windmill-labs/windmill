-- Add up migration script here
-- Switch draft ownership from per-workspace `username` (FK to `usr`) to
-- per-instance `email` (FK to `password`). Email is always populated on
-- the authed session (including super-admins viewing foreign workspaces
-- they aren't members of), so the FK no longer blocks legitimate writes.
ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_usr_fkey;

DROP INDEX IF EXISTS draft_pkey_with_user;

ALTER TABLE draft RENAME COLUMN username TO email;
ALTER TABLE draft ALTER COLUMN email TYPE VARCHAR(255);

UPDATE draft
SET email = usr.email
FROM usr
WHERE draft.workspace_id = usr.workspace_id
  AND draft.email = usr.username;

ALTER TABLE draft
    ADD CONSTRAINT draft_password_fkey
    FOREIGN KEY (email)
    REFERENCES password(email)
    ON DELETE CASCADE
    ON UPDATE CASCADE;

CREATE UNIQUE INDEX draft_pkey_with_user
    ON draft (workspace_id, path, typ, email)
    WHERE email IS NOT NULL;
