DROP INDEX IF EXISTS idx_variable_expiration_due;

COMMENT ON COLUMN variable.expires_at IS NULL;

ALTER TABLE variable
    DROP COLUMN IF EXISTS value_expires_at,
    DROP COLUMN IF EXISTS expiration_dispatched_for;
