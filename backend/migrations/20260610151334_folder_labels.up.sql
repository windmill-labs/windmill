-- Add up migration script here
ALTER TABLE folder ADD COLUMN labels text[];
