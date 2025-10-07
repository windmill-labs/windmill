-- Add up migration script here

-- Extend MESSAGE_TYPE enum to include 'system' and 'tool'
ALTER TYPE MESSAGE_TYPE ADD VALUE 'system';
ALTER TYPE MESSAGE_TYPE ADD VALUE 'tool';

-- Add step_name column to flow_conversation_message table
ALTER TABLE flow_conversation_message
ADD COLUMN step_name VARCHAR(255);