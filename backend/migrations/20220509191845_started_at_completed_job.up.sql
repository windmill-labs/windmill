-- Add up migration script here
ALTER TABLE completed_job
ADD COLUMN started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();

