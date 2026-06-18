-- Add up migration script here
ALTER TABLE v2_job_queue
    ADD COLUMN IF NOT EXISTS worker VARCHAR(255),
    ADD COLUMN IF NOT EXISTS extras JSONB;
ALTER TABLE v2_job_queue RENAME COLUMN parent_job TO __parent_job;
ALTER TABLE v2_job_queue RENAME COLUMN created_by TO __created_by;
ALTER TABLE v2_job_queue RENAME COLUMN script_hash TO __script_hash;
ALTER TABLE v2_job_queue RENAME COLUMN script_path TO __script_path;
ALTER TABLE v2_job_queue RENAME COLUMN args TO __args;
ALTER TABLE v2_job_queue RENAME COLUMN logs TO __logs;
ALTER TABLE v2_job_queue RENAME COLUMN raw_code TO __raw_code;
ALTER TABLE v2_job_queue RENAME COLUMN canceled TO __canceled;
ALTER TABLE v2_job_queue RENAME COLUMN last_ping TO __last_ping;
ALTER TABLE v2_job_queue RENAME COLUMN job_kind TO __job_kind;
ALTER TABLE v2_job_queue RENAME COLUMN env_id TO __env_id;
ALTER TABLE v2_job_queue RENAME COLUMN schedule_path TO __schedule_path;
ALTER TABLE v2_job_queue RENAME COLUMN permissioned_as TO __permissioned_as;
ALTER TABLE v2_job_queue RENAME COLUMN flow_status TO __flow_status;
ALTER TABLE v2_job_queue RENAME COLUMN raw_flow TO __raw_flow;
ALTER TABLE v2_job_queue RENAME COLUMN is_flow_step TO __is_flow_step;
ALTER TABLE v2_job_queue RENAME COLUMN language TO __language;
ALTER TABLE v2_job_queue RENAME COLUMN same_worker TO __same_worker;
ALTER TABLE v2_job_queue RENAME COLUMN raw_lock TO __raw_lock;
ALTER TABLE v2_job_queue RENAME COLUMN pre_run_error TO __pre_run_error;
ALTER TABLE v2_job_queue RENAME COLUMN email TO __email;
ALTER TABLE v2_job_queue RENAME COLUMN visible_to_owner TO __visible_to_owner;
ALTER TABLE v2_job_queue RENAME COLUMN mem_peak TO __mem_peak;
ALTER TABLE v2_job_queue RENAME COLUMN root_job TO __root_job;
ALTER TABLE v2_job_queue RENAME COLUMN leaf_jobs TO __leaf_jobs;
ALTER TABLE v2_job_queue RENAME COLUMN concurrent_limit TO __concurrent_limit;
ALTER TABLE v2_job_queue RENAME COLUMN concurrency_time_window_s TO __concurrency_time_window_s;
ALTER TABLE v2_job_queue RENAME COLUMN timeout TO __timeout;
ALTER TABLE v2_job_queue RENAME COLUMN flow_step_id TO __flow_step_id;
ALTER TABLE v2_job_queue RENAME COLUMN cache_ttl TO __cache_ttl;
