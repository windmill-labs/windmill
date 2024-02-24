-- Add up migration script here
create index if not exists ix_completed_job_workspace_id_created_at 
on completed_job(workspace_id, created_at desc, started_at desc, is_skipped, is_flow_step, job_kind);