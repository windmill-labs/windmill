-- Add up migration script here
ALTER TABLE schedule
	ADD COLUMN description TEXT NULL,
	ALTER COLUMN on_failure_extra_args SET DATA TYPE jsonb USING on_failure_extra_args::jsonb,
	ALTER COLUMN on_success_extra_args SET DATA TYPE jsonb USING on_success_extra_args::jsonb,
	ALTER COLUMN on_recovery_extra_args SET DATA TYPE jsonb USING on_recovery_extra_args::jsonb;
