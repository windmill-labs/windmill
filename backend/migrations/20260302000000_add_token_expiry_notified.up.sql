-- Tracks pending expiry notifications: row exists = not yet notified.
-- Deleted once the notification is sent. Orphaned rows are harmless (filtered out by the join).
CREATE TABLE token_expiry_notification (
    token VARCHAR(255) PRIMARY KEY,
    expiration TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_token_expiry_notification_expiration ON token_expiry_notification (expiration);
