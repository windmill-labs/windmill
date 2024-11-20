-- Step 1: Add the new column 'acknowledged' to the 'alerts' table
ALTER TABLE alerts
ADD COLUMN acknowledged BOOLEAN DEFAULT NULL;

-- Step 2: Update all existing rows to set 'acknowledged' to true
-- we don't want to pop up notifications to all users after migrations
-- but only show new alerts from the point of the upgrade
UPDATE alerts
SET acknowledged = TRUE;