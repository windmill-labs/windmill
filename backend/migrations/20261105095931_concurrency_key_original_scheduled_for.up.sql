-- The job's scheduled_for before a concurrency limit first deferred it. Set only on a
-- deferred job, so a job scheduled for later is never mistaken for one waiting on a slot.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS original_scheduled_for TIMESTAMPTZ;
