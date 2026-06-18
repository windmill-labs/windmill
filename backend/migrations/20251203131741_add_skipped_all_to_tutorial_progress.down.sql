-- Remove skipped_all column from tutorial_progress table
ALTER TABLE tutorial_progress 
DROP COLUMN skipped_all;
