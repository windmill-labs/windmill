-- Add down migration script here
DROP TABLE trigger;
DROP TYPE trigger_kind;

ALTER TABLE script DROP COLUMN has_preprocessor;