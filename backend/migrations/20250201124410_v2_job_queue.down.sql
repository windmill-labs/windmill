-- Add down migration script here
ALTER TABLE v2_job_queue
    DROP COLUMN worker CASCADE,
    DROP COLUMN extras CASCADE;
ALTER TABLE v2_job_queue RENAME COLUMN __parent_job TO parent_job;
ALTER TABLE v2_job_queue RENAME COLUMN __created_by TO created_by;
ALTER TABLE v2_job_queue RENAME COLUMN __script_hash TO script_hash;
ALTER TABLE v2_job_queue RENAME COLUMN __script_path TO script_path;
ALTER TABLE v2_job_queue RENAME COLUMN __args TO args;
ALTER TABLE v2_job_queue RENAME COLUMN __logs TO logs;
ALTER TABLE v2_job_queue RENAME COLUMN __raw_code TO raw_code;
ALTER TABLE v2_job_queue RENAME COLUMN __canceled TO canceled;
ALTER TABLE v2_job_queue RENAME COLUMN __last_ping TO last_ping;
ALTER TABLE v2_job_queue RENAME COLUMN __job_kind TO job_kind;
ALTER TABLE v2_job_queue RENAME COLUMN __env_id TO env_id;
ALTER TABLE v2_job_queue RENAME COLUMN __schedule_path TO schedule_path;
ALTER TABLE v2_job_queue RENAME COLUMN __permissioned_as TO permissioned_as;
ALTER TABLE v2_job_queue RENAME COLUMN __flow_status TO flow_status;
ALTER TABLE v2_job_queue RENAME COLUMN __raw_flow TO raw_flow;
ALTER TABLE v2_job_queue RENAME COLUMN __is_flow_step TO is_flow_step;
ALTER TABLE v2_job_queue RENAME COLUMN __language TO language;
ALTER TABLE v2_job_queue RENAME COLUMN __same_worker TO same_worker;
ALTER TABLE v2_job_queue RENAME COLUMN __raw_lock TO raw_lock;
ALTER TABLE v2_job_queue RENAME COLUMN __pre_run_error TO pre_run_error;
ALTER TABLE v2_job_queue RENAME COLUMN __email TO email;
ALTER TABLE v2_job_queue RENAME COLUMN __visible_to_owner TO visible_to_owner;
ALTER TABLE v2_job_queue RENAME COLUMN __mem_peak TO mem_peak;
ALTER TABLE v2_job_queue RENAME COLUMN __root_job TO root_job;
ALTER TABLE v2_job_queue RENAME COLUMN __leaf_jobs TO leaf_jobs;
ALTER TABLE v2_job_queue RENAME COLUMN __concurrent_limit TO concurrent_limit;
ALTER TABLE v2_job_queue RENAME COLUMN __concurrency_time_window_s TO concurrency_time_window_s;
ALTER TABLE v2_job_queue RENAME COLUMN __timeout TO timeout;
ALTER TABLE v2_job_queue RENAME COLUMN __flow_step_id TO flow_step_id;
ALTER TABLE v2_job_queue RENAME COLUMN __cache_ttl TO cache_ttl;
