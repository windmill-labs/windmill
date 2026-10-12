-- AES-256-GCM ciphertext is 32 bytes (magic, nonce, tag) longer than legacy CBC before base64,
-- so the longest CBC value that fits 15000 chars is exactly 15040 chars once re-encrypted.
ALTER TABLE variable ALTER COLUMN value TYPE VARCHAR(15040);
