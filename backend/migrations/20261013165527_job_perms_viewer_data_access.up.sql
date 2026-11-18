-- Set by the server on a job pushed for an app runnable whose policy says it reads data as its
-- viewer: data table connections that name no role and object storage permission rules then go by
-- end_user_email instead of the job's own identity.
ALTER TABLE job_perms ADD COLUMN IF NOT EXISTS viewer_data_access BOOLEAN NOT NULL DEFAULT false;
