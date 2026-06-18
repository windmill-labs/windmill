-- Hashed values cannot be reversed; truncate to avoid silent join mismatches
-- with the old code that compared plaintext token_expiry_notification.token against token.token.
TRUNCATE token_expiry_notification;

ALTER TABLE token_expiry_notification RENAME COLUMN token_hash TO token;
