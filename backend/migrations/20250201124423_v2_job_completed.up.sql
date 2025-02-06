-- Add up migration script here
CREATE TYPE job_status AS ENUM ('success', 'failure', 'canceled', 'skipped');
ALTER TABLE v2_job_completed
    ADD COLUMN IF NOT EXISTS status job_status,
    ADD COLUMN IF NOT EXISTS completed_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS worker VARCHAR(255),
    ADD COLUMN IF NOT EXISTS workflow_as_code_status JSONB,
    ADD COLUMN IF NOT EXISTS result_columns TEXT[],
    ADD COLUMN IF NOT EXISTS retries UUID[],
    ADD COLUMN IF NOT EXISTS extras JSONB;
ALTER TABLE v2_job_completed RENAME COLUMN mem_peak TO memory_peak;
ALTER TABLE v2_job_completed RENAME COLUMN parent_job TO __parent_job;
ALTER TABLE v2_job_completed RENAME COLUMN created_by TO __created_by;
ALTER TABLE v2_job_completed RENAME COLUMN created_at TO __created_at;
ALTER TABLE v2_job_completed RENAME COLUMN success TO __success;
ALTER TABLE v2_job_completed RENAME COLUMN script_hash TO __script_hash;
ALTER TABLE v2_job_completed RENAME COLUMN script_path TO __script_path;
ALTER TABLE v2_job_completed RENAME COLUMN args TO __args;
ALTER TABLE v2_job_completed RENAME COLUMN logs TO __logs;
ALTER TABLE v2_job_completed RENAME COLUMN raw_code TO __raw_code;
ALTER TABLE v2_job_completed RENAME COLUMN canceled TO __canceled;
ALTER TABLE v2_job_completed RENAME COLUMN job_kind TO __job_kind;
ALTER TABLE v2_job_completed RENAME COLUMN env_id TO __env_id;
ALTER TABLE v2_job_completed RENAME COLUMN schedule_path TO __schedule_path;
ALTER TABLE v2_job_completed RENAME COLUMN permissioned_as TO __permissioned_as;
ALTER TABLE v2_job_completed RENAME COLUMN raw_flow TO __raw_flow;
ALTER TABLE v2_job_completed RENAME COLUMN is_flow_step TO __is_flow_step;
ALTER TABLE v2_job_completed RENAME COLUMN language TO __language;
ALTER TABLE v2_job_completed RENAME COLUMN is_skipped TO __is_skipped;
ALTER TABLE v2_job_completed RENAME COLUMN raw_lock TO __raw_lock;
ALTER TABLE v2_job_completed RENAME COLUMN email TO __email;
ALTER TABLE v2_job_completed RENAME COLUMN visible_to_owner TO __visible_to_owner;
ALTER TABLE v2_job_completed RENAME COLUMN tag TO __tag;
ALTER TABLE v2_job_completed RENAME COLUMN priority TO __priority;
