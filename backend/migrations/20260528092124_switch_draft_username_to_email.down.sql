-- Add down migration script here
ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_password_fkey;
DROP INDEX IF EXISTS draft_pkey_with_user;

-- Best-effort: map back to a workspace username if the email matches a
-- single `usr` row. Drafts whose email doesn't resolve to a usr row are
-- dropped — the previous schema had no way to represent them.
UPDATE draft
SET email = usr.username
FROM usr
WHERE draft.workspace_id = usr.workspace_id
  AND draft.email = usr.email;

DELETE FROM draft
WHERE email IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM usr
    WHERE usr.workspace_id = draft.workspace_id
      AND usr.username = draft.email
  );

ALTER TABLE draft RENAME COLUMN email TO username;
ALTER TABLE draft ALTER COLUMN username TYPE VARCHAR(50);

ALTER TABLE draft
    ADD CONSTRAINT draft_usr_fkey
    FOREIGN KEY (workspace_id, username)
    REFERENCES usr(workspace_id, username)
    ON DELETE CASCADE
    ON UPDATE CASCADE;

CREATE UNIQUE INDEX draft_pkey_with_user
    ON draft (workspace_id, path, typ, username)
    WHERE username IS NOT NULL;
