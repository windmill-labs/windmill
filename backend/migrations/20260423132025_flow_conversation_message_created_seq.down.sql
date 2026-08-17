DROP INDEX IF EXISTS idx_conversation_message_conversation_created_seq;

CREATE INDEX IF NOT EXISTS idx_conversation_message_conversation_time
    ON flow_conversation_message(conversation_id, created_at DESC);

ALTER TABLE flow_conversation_message
    DROP CONSTRAINT IF EXISTS flow_conversation_message_created_seq_key;

ALTER TABLE flow_conversation_message
    DROP COLUMN IF EXISTS created_seq;
