-- Rename token -> token_hash for clarity: the column stores a hash, not a plaintext token.
ALTER TABLE token_expiry_notification RENAME COLUMN token TO token_hash;
