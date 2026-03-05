-- Step 1: Add columns, backfill, build indexes.
-- This migration does the heavy work but avoids ACCESS EXCLUSIVE during index build
-- by creating the unique index first, then using it for the PK swap in the next migration.

-- Add new columns (instant metadata change)
ALTER TABLE token ADD COLUMN token_hash VARCHAR(64);
ALTER TABLE token ADD COLUMN token_prefix VARCHAR(10);

-- Backfill existing tokens using built-in sha256() (no extension needed).
-- Takes ROW EXCLUSIVE lock — concurrent reads and non-token writes proceed normally.
UPDATE token
SET token_hash = encode(sha256(token::bytea), 'hex'),
    token_prefix = substring(token for 10)
WHERE token_hash IS NULL;

-- Mark NOT NULL (instant on PG 12+ when all rows already satisfy the constraint)
ALTER TABLE token ALTER COLUMN token_hash SET NOT NULL;
ALTER TABLE token ALTER COLUMN token_prefix SET NOT NULL;

-- Build the unique index that the next migration will promote to PK.
-- Takes SHARE lock (reads OK, writes wait) but only for the duration of the build,
-- which is fast since token tables are typically small.
CREATE UNIQUE INDEX token_hash_unique ON token (token_hash);

-- Index on prefix for deletion/listing
CREATE INDEX idx_token_prefix ON token (token_prefix);
