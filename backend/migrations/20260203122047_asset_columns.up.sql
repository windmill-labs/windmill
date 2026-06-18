-- Add columns field to asset table to store column-level access information
-- This is a JSONB map of column name to access type (r, w, or rw)
ALTER TABLE asset ADD COLUMN columns JSONB;
