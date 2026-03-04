-- Reverse of step 1: drop indexes and columns

DROP INDEX IF EXISTS idx_token_prefix;
DROP INDEX IF EXISTS token_hash_unique;

ALTER TABLE token DROP COLUMN token_hash;
ALTER TABLE token DROP COLUMN token_prefix;
