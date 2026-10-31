ALTER TABLE variable DROP CONSTRAINT IF EXISTS variable_oauth_no_value_expiry;
DROP INDEX IF EXISTS idx_variable_expiration_due;

COMMENT ON COLUMN variable.expires_at IS NULL;

ALTER TABLE variable
    DROP COLUMN IF EXISTS value_valid_until,
    DROP COLUMN IF EXISTS expiration_dispatched_for,
    DROP COLUMN IF EXISTS expiration_notified_at;
