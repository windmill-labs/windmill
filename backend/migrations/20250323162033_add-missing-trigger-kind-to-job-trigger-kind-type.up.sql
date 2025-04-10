-- Add up migration script here
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'sqs';
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'gcp';
