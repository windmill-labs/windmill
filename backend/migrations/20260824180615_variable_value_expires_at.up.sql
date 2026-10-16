-- `value_expires_at` is when the value stored in the variable stops working; the row itself
-- lives on. `expires_at` keeps its unrelated meaning — delete the row — so no existing writer
-- changes meaning under a rolling deploy and no data is rewritten here.
ALTER TABLE variable ADD COLUMN IF NOT EXISTS value_expires_at TIMESTAMP WITH TIME ZONE;

COMMENT ON COLUMN variable.expires_at IS
    'Garbage collection deadline: the row is deleted once passed. Set by the ephemeral secret_arg mint. Not the value''s expiry — see value_expires_at.';
COMMENT ON COLUMN variable.value_expires_at IS
    'When the value stops working. Drives the workspace variable expiration handler; never deletes the row.';

-- Holds the `value_expires_at` that was dispatched, rather than a plain "was dispatched" flag,
-- so that moving the date re-arms the handler by construction — no writer has to remember to
-- clear anything.
ALTER TABLE variable
    ADD COLUMN IF NOT EXISTS expiration_dispatched_for TIMESTAMP WITH TIME ZONE;

-- When the handler last ran for this variable, which paces its reminders.
ALTER TABLE variable
    ADD COLUMN IF NOT EXISTS expiration_notified_at TIMESTAMP WITH TIME ZONE;

-- Serves the sweep's due predicate. Partial so it stays the size of the opted-in set rather
-- than the whole variable table. Not partial on `expiration_dispatched_for`: reminders keep a
-- dispatched variable due until it expires.
CREATE INDEX IF NOT EXISTS idx_variable_expiration_due ON variable (value_expires_at)
    WHERE value_expires_at IS NOT NULL;

-- An OAuth-linked value is refreshed by Windmill itself, so a rotation date on it would only
-- announce an expiry that never comes.
ALTER TABLE variable ADD CONSTRAINT variable_oauth_no_value_expiry
    CHECK (account IS NULL OR value_expires_at IS NULL);
