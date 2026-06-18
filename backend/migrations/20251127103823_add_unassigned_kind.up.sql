-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'unassigned_script';
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'unassigned_flow';
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'unassigned_singlestepflow';
