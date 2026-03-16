ALTER TABLE native_trigger ADD COLUMN webhook_token_prefix VARCHAR(10) NOT NULL DEFAULT '';

-- Backfill prefix from the token table
UPDATE native_trigger nt
SET webhook_token_prefix = t.token_prefix
FROM token t
WHERE t.token_hash = nt.webhook_token_hash;

ALTER TABLE native_trigger DROP COLUMN IF EXISTS webhook_token_hash;
