-- Add webhook_token_hash to native_trigger for safe token lookups/deletes.
-- The existing webhook_token_prefix (10 chars) is kept for display but is not
-- unique enough for deletion.

ALTER TABLE native_trigger ADD COLUMN webhook_token_hash VARCHAR(64);

-- Backfill from the token table
UPDATE native_trigger nt
SET webhook_token_hash = t.token_hash
FROM token t
WHERE t.token_prefix = nt.webhook_token_prefix;

-- New triggers will always have a hash, but existing ones might not
-- if their token was already deleted. Leave those NULL.
