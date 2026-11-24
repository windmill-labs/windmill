ALTER TABLE password
    DROP COLUMN IF EXISTS totp_secret,
    DROP COLUMN IF EXISTS totp_pending_secret,
    DROP COLUMN IF EXISTS totp_last_step,
    DROP COLUMN IF EXISTS totp_recovery_codes,
    DROP COLUMN IF EXISTS totp_failed_attempts,
    DROP COLUMN IF EXISTS totp_failed_since;
