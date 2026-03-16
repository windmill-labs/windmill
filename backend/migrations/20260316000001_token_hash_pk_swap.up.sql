-- Step 2: Swap PK and update trigger.
-- All operations here are instant metadata changes (no data/index rebuild).
-- The ACCESS EXCLUSIVE lock is held for only milliseconds.

-- Swap primary key: drop old, promote existing unique index (instant)
ALTER TABLE token DROP CONSTRAINT token_pkey;
ALTER TABLE token ADD CONSTRAINT token_pkey PRIMARY KEY USING INDEX token_hash_unique;

-- Make old token column nullable (no longer written for new tokens)
ALTER TABLE token ALTER COLUMN token DROP NOT NULL;

-- Update the cache invalidation trigger to send prefix instead of plaintext
CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', OLD.token_prefix);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
