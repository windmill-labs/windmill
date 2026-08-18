-- Add down migration script here
DROP FUNCTION folder_labels(text, text);
ALTER TABLE folder DROP COLUMN labels;
