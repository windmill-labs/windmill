ALTER TABLE password
    DROP COLUMN IF EXISTS totp_secret,
    DROP COLUMN IF EXISTS totp_pending_secret,
    DROP COLUMN IF EXISTS totp_last_step;
