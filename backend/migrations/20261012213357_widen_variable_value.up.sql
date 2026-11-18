-- AES-256-GCM ciphertext is 32 bytes (magic, nonce, tag) longer than legacy CBC before base64,
-- so the longest CBC value that fits 15000 chars is exactly 15040 chars once re-encrypted.
ALTER TABLE variable ALTER COLUMN value TYPE VARCHAR(15040);

-- Only GCM secrets (base64 of the `wmA2` magic starts `d21BM`) may use the extra room: a longer
-- CBC value could never be re-encrypted to fit, and would fail every rotation of its workspace key.
ALTER TABLE variable ADD CONSTRAINT variable_value_length
    CHECK (length(value) <= 15000 OR (is_secret AND value LIKE 'd21BM%'));
