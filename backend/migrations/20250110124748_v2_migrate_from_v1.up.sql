-- Add up migration script here

-- Set new columns in `v2_job_completed`:
UPDATE v2_job_completed
SET completed_at = started_at + (interval '1 millisecond' * duration_ms),
    status = CASE
        WHEN __is_skipped THEN 'skipped'::job_status
        WHEN __canceled THEN 'canceled'::job_status
        WHEN __success THEN 'success'::job_status
    ELSE 'failure'::job_status END
WHERE status IS NULL;

-- Insert missing `v2_job` rows from `v2_job_queue`:
INSERT INTO v2_job (
    id, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email,
    kind, runnable_id, runnable_path, parent_job,
    script_lang,
    flow_step_id, flow_root_job,
    schedule_path,
    tag, same_worker, visible_to_owner, concurrent_limit, concurrency_time_window_s, cache_ttl, timeout, priority,
    args, pre_run_error,
    raw_code, raw_lock, raw_flow
) SELECT
    id, workspace_id, created_at, __created_by, __permissioned_as, __email,
    __job_kind, __script_hash, __script_path, __parent_job,
    __language,
    __flow_step_id, __root_job,
    __schedule_path,
    tag, __same_worker, __visible_to_owner, __concurrent_limit, __concurrency_time_window_s,
    __cache_ttl, __timeout, priority,
    __args, __pre_run_error,
    __raw_code, __raw_lock, __raw_flow
FROM v2_job_queue
WHERE NOT EXISTS (SELECT 1 FROM v2_job WHERE v2_job.id = v2_job_queue.id);

-- Insert missing `v2_job` rows from `v2_job_completed`:
INSERT INTO v2_job (
    id, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email,
    kind, runnable_id, runnable_path, parent_job,
    script_lang,
    schedule_path,
    tag, visible_to_owner, priority,
    args,
    raw_code, raw_lock, raw_flow
) SELECT
    id, workspace_id, __created_at, __created_by, __permissioned_as, __email,
    __job_kind, __script_hash, __script_path, __parent_job,
    __language,
    __schedule_path,
    __tag, __visible_to_owner, __priority,
    __args,
    __raw_code, __raw_lock, __raw_flow
FROM v2_job_completed
WHERE NOT EXISTS (SELECT 1 FROM v2_job WHERE v2_job.id = v2_job_completed.id);

-- Set existing `v2_job` rows from `v2_job_queue`:
UPDATE v2_job SET
    created_at = v2_job_queue.created_at,
    created_by = v2_job_queue.__created_by,
    permissioned_as = v2_job_queue.__permissioned_as,
    permissioned_as_email = v2_job_queue.__email,
    kind = v2_job_queue.__job_kind,
    runnable_id = v2_job_queue.__script_hash,
    runnable_path = v2_job_queue.__script_path,
    parent_job = v2_job_queue.__parent_job,
    script_lang = v2_job_queue.__language,
    flow_step_id = v2_job_queue.__flow_step_id,
    flow_root_job = v2_job_queue.__root_job,
    schedule_path = v2_job_queue.__schedule_path,
    tag = v2_job_queue.tag,
    same_worker = v2_job_queue.__same_worker,
    visible_to_owner = v2_job_queue.__visible_to_owner,
    concurrent_limit = v2_job_queue.__concurrent_limit,
    concurrency_time_window_s = v2_job_queue.__concurrency_time_window_s,
    cache_ttl = v2_job_queue.__cache_ttl,
    timeout = v2_job_queue.__timeout,
    priority = v2_job_queue.priority,
    args = v2_job_queue.__args,
    pre_run_error = v2_job_queue.__pre_run_error,
    raw_code = COALESCE(v2_job.raw_code, v2_job_queue.__raw_code),
    raw_lock = COALESCE(v2_job.raw_lock, v2_job_queue.__raw_lock),
    raw_flow = COALESCE(v2_job.raw_flow, v2_job_queue.__raw_flow)
FROM v2_job_queue
WHERE v2_job.id = v2_job_queue.id AND v2_job.created_by = 'missing';

-- Set existing `v2_job` rows from `v2_job_completed`:
UPDATE v2_job SET
    created_at = v2_job_completed.__created_at,
    created_by = v2_job_completed.__created_by,
    permissioned_as = v2_job_completed.__permissioned_as,
    permissioned_as_email = v2_job_completed.__email,
    kind = v2_job_completed.__job_kind,
    runnable_id = v2_job_completed.__script_hash,
    runnable_path = v2_job_completed.__script_path,
    parent_job = v2_job_completed.__parent_job,
    script_lang = v2_job_completed.__language,
    schedule_path = v2_job_completed.__schedule_path,
    tag = v2_job_completed.__tag,
    visible_to_owner = v2_job_completed.__visible_to_owner,
    priority = v2_job_completed.__priority,
    args = v2_job_completed.__args,
    raw_code = COALESCE(v2_job.raw_code, v2_job_completed.__raw_code),
    raw_lock = COALESCE(v2_job.raw_lock, v2_job_completed.__raw_lock),
    raw_flow = COALESCE(v2_job.raw_flow, v2_job_completed.__raw_flow)
FROM v2_job_completed
WHERE v2_job.id = v2_job_completed.id AND v2_job.created_by = 'missing';

-- Migrate `v2_job_queue` moved columns to `v2_job_runtime`:
INSERT INTO v2_job_runtime (id, ping, memory_peak)
SELECT id, __last_ping, __mem_peak
FROM v2_job_queue
WHERE NOT running AND __last_ping IS NOT NULL OR __mem_peak IS NOT NULL
    -- Locked ones will sync within triggers
    FOR UPDATE SKIP LOCKED
ON CONFLICT (id) DO NOTHING;

-- Migrate `v2_job_queue` moved columns to `v2_flow_job_runtime`:
INSERT INTO v2_job_flow_runtime (id, flow_status, leaf_jobs)
SELECT id, __flow_status, __leaf_jobs
FROM v2_job_queue
WHERE NOT running AND __flow_status IS NOT NULL OR __leaf_jobs IS NOT NULL
   -- Locked ones will sync within triggers
    FOR UPDATE SKIP LOCKED
ON CONFLICT (id) DO NOTHING;

-- Migrate old `v2_job_queue.__logs` to `job_logs`
INSERT INTO job_logs (job_id, workspace_id, logs)
SELECT id, workspace_id, __logs
FROM v2_job_queue
WHERE __logs IS NOT NULL
ON CONFLICT (job_id) DO UPDATE SET
    logs = CONCAT(job_logs.logs, EXCLUDED.logs)
;

-- Migrate old `v2_job_completed.__logs` to `job_logs`
INSERT INTO job_logs (job_id, workspace_id, logs)
SELECT id, workspace_id, __logs
FROM v2_job_completed
WHERE __logs IS NOT NULL AND __logs != '##DELETED##'
ON CONFLICT (job_id) DO UPDATE SET
    logs = CONCAT(job_logs.logs, EXCLUDED.logs)
;
