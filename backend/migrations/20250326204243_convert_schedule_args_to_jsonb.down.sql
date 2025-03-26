-- Add down migration script here
ALTER TABLE schedule
ALTER COLUMN on_failure_extra_args SET DATA TYPE json USING on_failure_extra_args::json,
ALTER COLUMN on_success_extra_args SET DATA TYPE json USING on_success_extra_args::json,
ALTER COLUMN on_recovery_extra_args SET DATA TYPE json USING on_recovery_extra_args::json;
