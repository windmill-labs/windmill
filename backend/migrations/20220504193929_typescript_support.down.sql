-- Add down migration script here
DROP TYPE SCRIPT_LANG;

ALTER TABLE script
DROP COLUMN language SCRIPT_LANG;

