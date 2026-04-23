DROP INDEX IF EXISTS idx_conversation_message_conversation_created_seq;

ALTER TABLE flow_conversation_message
    DROP CONSTRAINT IF EXISTS flow_conversation_message_created_seq_key;

ALTER TABLE flow_conversation_message
    DROP COLUMN IF EXISTS created_seq;
