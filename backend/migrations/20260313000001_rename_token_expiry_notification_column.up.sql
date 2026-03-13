-- Convert existing plaintext token values to SHA-256 hashes, then rename the column.
-- Hashes are 64-char hex strings; anything shorter is a plaintext token that needs hashing.
UPDATE token_expiry_notification
SET token = encode(sha256(token::bytea), 'hex')
WHERE length(token) != 64;

ALTER TABLE token_expiry_notification RENAME COLUMN token TO token_hash;
