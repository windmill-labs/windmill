-- Add webhook_token_hash to native_trigger for safe token lookups/deletes,
-- and drop webhook_token_prefix which is no longer needed.

ALTER TABLE native_trigger ADD COLUMN webhook_token_hash VARCHAR(64);

-- Backfill from the token table
UPDATE native_trigger nt
SET webhook_token_hash = t.token_hash
FROM token t
WHERE t.token_prefix = nt.webhook_token_prefix;

-- Remove orphaned triggers whose tokens no longer exist
DO $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM native_trigger WHERE webhook_token_hash IS NULL;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    IF deleted_count > 0 THEN
        RAISE NOTICE 'Deleted % orphaned native_trigger(s) with no matching token', deleted_count;
    END IF;
END $$;

ALTER TABLE native_trigger ALTER COLUMN webhook_token_hash SET NOT NULL;

ALTER TABLE native_trigger DROP COLUMN webhook_token_prefix;
