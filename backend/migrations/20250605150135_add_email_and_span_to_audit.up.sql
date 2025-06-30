-- Add email and span columns to audit table
ALTER TABLE audit ADD COLUMN email VARCHAR(255);
ALTER TABLE audit ADD COLUMN span VARCHAR(255);
