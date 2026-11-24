-- When the concurrency limit let in a job it had held back: the time the job was last due.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS released_at TIMESTAMPTZ;
