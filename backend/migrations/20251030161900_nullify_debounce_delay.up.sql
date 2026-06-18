-- Add up migration script here
UPDATE script 
SET debounce_delay_s = NULL 
WHERE debounce_delay_s = 0;
