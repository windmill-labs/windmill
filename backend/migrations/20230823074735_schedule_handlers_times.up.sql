-- Add up migration script here
ALTER TABLE schedule
ADD COLUMN on_failure_times INTEGER,
ADD COLUMN on_failure_exact BOOLEAN,
ADD COLUMN on_failure_extra_args json,
ADD COLUMN on_recovery_times INTEGER,
ADD COLUMN on_recovery_extra_args json;