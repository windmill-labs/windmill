-- Remove added_via tracking
DROP INDEX idx_usr_added_via;
ALTER TABLE usr DROP COLUMN added_via;
