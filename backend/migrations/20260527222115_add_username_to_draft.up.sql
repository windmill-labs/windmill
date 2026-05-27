-- Add up migration script here
ALTER TABLE draft ADD COLUMN username VARCHAR(50);

ALTER TABLE draft
    ADD CONSTRAINT draft_usr_fkey
    FOREIGN KEY (workspace_id, username)
    REFERENCES usr(workspace_id, username)
    ON DELETE CASCADE
    ON UPDATE CASCADE;

ALTER TABLE draft DROP CONSTRAINT draft_pkey;

CREATE UNIQUE INDEX draft_pkey_with_user
    ON draft (workspace_id, path, typ, username)
    WHERE username IS NOT NULL;

CREATE UNIQUE INDEX draft_pkey_legacy
    ON draft (workspace_id, path, typ)
    WHERE username IS NULL;
