-- Track whether an expiry notification has been sent for this token
ALTER TABLE token ADD COLUMN expiry_notified BOOLEAN NOT NULL DEFAULT FALSE;
