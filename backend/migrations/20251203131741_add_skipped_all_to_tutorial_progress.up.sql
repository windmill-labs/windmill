-- Add skipped_all column to tutorial_progress table
ALTER TABLE tutorial_progress 
ADD COLUMN skipped_all BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN tutorial_progress.skipped_all IS 'Indicates if the user has skipped all tutorials (vs completing them all)';
