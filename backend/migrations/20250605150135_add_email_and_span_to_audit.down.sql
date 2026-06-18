-- Remove email and span columns from audit table
ALTER TABLE audit DROP COLUMN email;
ALTER TABLE audit DROP COLUMN span;
