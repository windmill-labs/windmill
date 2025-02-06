-- Add down migration script here
ALTER TABLE v2_job_completed
    DROP COLUMN status CASCADE,
    DROP COLUMN completed_at CASCADE,
    DROP COLUMN worker CASCADE,
    DROP COLUMN workflow_as_code_status CASCADE,
    DROP COLUMN result_columns CASCADE,
    DROP COLUMN retries CASCADE,
    DROP COLUMN extras CASCADE;
ALTER TABLE v2_job_completed RENAME COLUMN memory_peak TO mem_peak;
ALTER TABLE v2_job_completed RENAME COLUMN __parent_job TO parent_job;
ALTER TABLE v2_job_completed RENAME COLUMN __created_by TO created_by;
ALTER TABLE v2_job_completed RENAME COLUMN __created_at TO created_at;
ALTER TABLE v2_job_completed RENAME COLUMN __success TO success;
ALTER TABLE v2_job_completed RENAME COLUMN __script_hash TO script_hash;
ALTER TABLE v2_job_completed RENAME COLUMN __script_path TO script_path;
ALTER TABLE v2_job_completed RENAME COLUMN __args TO args;
ALTER TABLE v2_job_completed RENAME COLUMN __logs TO logs;
ALTER TABLE v2_job_completed RENAME COLUMN __raw_code TO raw_code;
ALTER TABLE v2_job_completed RENAME COLUMN __canceled TO canceled;
ALTER TABLE v2_job_completed RENAME COLUMN __job_kind TO job_kind;
ALTER TABLE v2_job_completed RENAME COLUMN __env_id TO env_id;
ALTER TABLE v2_job_completed RENAME COLUMN __schedule_path TO schedule_path;
ALTER TABLE v2_job_completed RENAME COLUMN __permissioned_as TO permissioned_as;
ALTER TABLE v2_job_completed RENAME COLUMN __raw_flow TO raw_flow;
ALTER TABLE v2_job_completed RENAME COLUMN __is_flow_step TO is_flow_step;
ALTER TABLE v2_job_completed RENAME COLUMN __language TO language;
ALTER TABLE v2_job_completed RENAME COLUMN __is_skipped TO is_skipped;
ALTER TABLE v2_job_completed RENAME COLUMN __raw_lock TO raw_lock;
ALTER TABLE v2_job_completed RENAME COLUMN __email TO email;
ALTER TABLE v2_job_completed RENAME COLUMN __visible_to_owner TO visible_to_owner;
ALTER TABLE v2_job_completed RENAME COLUMN __tag TO tag;
ALTER TABLE v2_job_completed RENAME COLUMN __priority TO priority;
DROP TYPE IF EXISTS job_status CASCADE;
