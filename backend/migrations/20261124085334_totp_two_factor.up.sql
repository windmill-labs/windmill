ALTER TABLE password
    ADD COLUMN IF NOT EXISTS totp_secret TEXT,
    ADD COLUMN IF NOT EXISTS totp_pending_secret TEXT,
    ADD COLUMN IF NOT EXISTS totp_last_step BIGINT;
