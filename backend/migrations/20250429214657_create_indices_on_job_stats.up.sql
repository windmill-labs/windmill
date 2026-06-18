-- Add up migration script here
CREATE INDEX IF NOT EXISTS job_stats_id ON job_stats (job_id);