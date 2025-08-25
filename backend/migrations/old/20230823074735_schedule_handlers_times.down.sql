-- Add down migration script here
ALTER TABLE schedule 
DROP COLUMN on_failure_times,
DROP COLUMN on_failure_exact,
DROP COLUMN on_failure_extra_args,
DROP COLUMN on_recovery_times,
DROP COLUMN on_recovery_extra_args;
