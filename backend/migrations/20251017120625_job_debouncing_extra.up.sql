-- Add up migration script here
ALTER TABLE script ADD COLUMN debounce_key VARCHAR(255);
ALTER TABLE script ADD COLUMN debounce_delay_s INTEGER;
