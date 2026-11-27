ALTER TABLE password
    ADD COLUMN IF NOT EXISTS totp_secret TEXT,
    ADD COLUMN IF NOT EXISTS totp_pending_secret TEXT,
    ADD COLUMN IF NOT EXISTS totp_last_step BIGINT,
    ADD COLUMN IF NOT EXISTS totp_recovery_codes TEXT[],
    ADD COLUMN IF NOT EXISTS totp_failed_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS totp_failed_since TIMESTAMPTZ;
