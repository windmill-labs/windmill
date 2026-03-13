-- Add webhook_token_hash to native_trigger for safe token lookups/deletes,
-- and drop webhook_token_prefix which is no longer needed.

ALTER TABLE native_trigger ADD COLUMN webhook_token_hash VARCHAR(64);

-- Backfill from the token table
UPDATE native_trigger nt
SET webhook_token_hash = t.token_hash
FROM token t
WHERE t.token_prefix = nt.webhook_token_prefix;

-- Remove orphaned triggers whose tokens no longer exist
DELETE FROM native_trigger WHERE webhook_token_hash IS NULL;

ALTER TABLE native_trigger ALTER COLUMN webhook_token_hash SET NOT NULL;

ALTER TABLE native_trigger DROP COLUMN webhook_token_prefix;
