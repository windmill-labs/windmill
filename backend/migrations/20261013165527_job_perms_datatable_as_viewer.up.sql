-- Set by the server on a job pushed for an app runnable whose policy says it connects to data
-- tables as its viewer. Read with end_user_email when an unnamed data table connection is resolved.
ALTER TABLE job_perms ADD COLUMN IF NOT EXISTS datatable_as_viewer BOOLEAN NOT NULL DEFAULT false;
