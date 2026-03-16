-- Reverse step 2: restore old PK and trigger

-- Restore the original trigger
CREATE OR REPLACE FUNCTION notify_token_invalidation()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_token_invalidation', OLD.token);
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Delete tokens created after migration that have no plaintext (cannot be restored)
DELETE FROM token WHERE token IS NULL;

-- Make token NOT NULL again
ALTER TABLE token ALTER COLUMN token SET NOT NULL;

-- Swap PK back: drop token_hash PK, restore token PK
ALTER TABLE token DROP CONSTRAINT token_pkey;
ALTER TABLE token ADD PRIMARY KEY (token);

-- Re-create the unique index on token_hash (was consumed by ADD CONSTRAINT ... USING INDEX)
CREATE UNIQUE INDEX token_hash_unique ON token (token_hash);
