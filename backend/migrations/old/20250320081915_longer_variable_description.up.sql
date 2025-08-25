-- Add up migration script here
ALTER TABLE variable 
ALTER COLUMN description TYPE VARCHAR(10000);