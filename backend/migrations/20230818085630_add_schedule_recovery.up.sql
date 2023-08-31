-- Add up migration script here
ALTER TABLE schedule ADD COLUMN on_recovery VARCHAR(1000);