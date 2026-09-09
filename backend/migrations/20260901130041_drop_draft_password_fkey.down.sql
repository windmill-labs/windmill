-- Drafts owned by a principal with no login account cannot exist under the constraint; drop them
-- before restoring it.
DELETE FROM draft
WHERE email IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM password WHERE password.email = draft.email);

ALTER TABLE draft
    ADD CONSTRAINT draft_password_fkey
    FOREIGN KEY (email)
    REFERENCES password(email)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
