-- Add dynamic_skip column to schedule table
-- This column stores the path to a script that validates scheduled datetimes
-- The handler receives the scheduled_for datetime and returns a boolean
ALTER TABLE schedule ADD COLUMN dynamic_skip VARCHAR(1000) DEFAULT NULL;
