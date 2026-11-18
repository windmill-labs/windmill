-- Add 'asset' as a job_trigger_kind so jobs that get dispatched as a
-- consequence of an upstream pipeline script writing an asset can be
-- attributed via v2_job.trigger_kind = 'asset'. The producer's runnable
-- path goes into v2_job.trigger.
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'asset';
