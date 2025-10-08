-- Remove step_name and success columns
ALTER TABLE flow_conversation_message DROP COLUMN IF EXISTS step_name;
ALTER TABLE flow_conversation_message DROP COLUMN IF EXISTS success;