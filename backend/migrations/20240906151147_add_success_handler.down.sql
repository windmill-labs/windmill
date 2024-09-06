-- Add down migration script here
ALTER TABLE schedule DROP COLUMN on_success, DROP COLUMN on_success_extra_args;