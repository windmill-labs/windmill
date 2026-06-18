-- Add up migration script here

-- Extend MESSAGE_TYPE enum to include 'tool'
ALTER TYPE MESSAGE_TYPE ADD VALUE 'tool';

-- Add step_name and success columns to flow_conversation_message table
ALTER TABLE flow_conversation_message ADD COLUMN step_name VARCHAR(255);
ALTER TABLE flow_conversation_message ADD COLUMN success BOOLEAN DEFAULT TRUE NOT NULL;