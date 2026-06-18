-- Add down migration script here

-- Drop indexes
DROP INDEX IF EXISTS idx_conversation_message_conversation_time;
DROP INDEX IF EXISTS idx_flow_conversation_workspace_path;

-- Drop tables (order matters due to foreign keys)
DROP TABLE IF EXISTS flow_conversation_message;
DROP TABLE IF EXISTS flow_conversation;

-- Drop enum
DROP TYPE IF EXISTS MESSAGE_TYPE;