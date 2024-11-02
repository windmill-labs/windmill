-- Add up migration script here
alter table queue drop constraint if exists queue_workspace_id_fkey;
alter table completed_job drop constraint if exists completed_job_workspace_id_fkey;
