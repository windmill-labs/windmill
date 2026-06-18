-- Table to track AI chat sessions and message counts for telemetry
CREATE TABLE IF NOT EXISTS ai_chat_usage (
    id BIGSERIAL PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL UNIQUE,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(255) NOT NULL,
    mode VARCHAR(50) NOT NULL,
    message_count INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ai_chat_usage_created_at ON ai_chat_usage (created_at);
