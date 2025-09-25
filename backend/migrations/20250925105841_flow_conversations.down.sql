-- Add down migration script here

-- Drop trigger and function
DROP TRIGGER IF EXISTS update_conversation_on_message ON flow_conversation_message;
DROP FUNCTION IF EXISTS update_conversation_timestamp();

-- Drop indexes
DROP INDEX IF EXISTS idx_conversation_message_job;
DROP INDEX IF EXISTS idx_conversation_message_conversation_time;
DROP INDEX IF EXISTS idx_flow_conversation_workspace_path;

-- Drop tables (order matters due to foreign keys)
DROP TABLE IF EXISTS flow_conversation_message;
DROP TABLE IF EXISTS flow_conversation;