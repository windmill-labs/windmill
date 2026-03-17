-- Convert existing plaintext token values to SHA-256 hashes, then rename the column.
UPDATE token_expiry_notification
SET token = encode(sha256(token::bytea), 'hex');

ALTER TABLE token_expiry_notification RENAME COLUMN token TO token_hash;
