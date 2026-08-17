ALTER TABLE flow_conversation_message
    ADD COLUMN created_seq BIGINT GENERATED ALWAYS AS IDENTITY;

ALTER TABLE flow_conversation_message
    ADD CONSTRAINT flow_conversation_message_created_seq_key UNIQUE (created_seq);

CREATE INDEX idx_conversation_message_conversation_created_seq
    ON flow_conversation_message(conversation_id, created_seq);

DROP INDEX IF EXISTS idx_conversation_message_conversation_time;
