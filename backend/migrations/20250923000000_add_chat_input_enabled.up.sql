-- Add up migration script here
ALTER TABLE flow ADD COLUMN chat_input_enabled BOOLEAN DEFAULT FALSE;