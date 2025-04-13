-- Add up migration script here
-- This trigger will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_queue_before_insert() RETURNS TRIGGER AS $$
DECLARE job v2_job;
BEGIN
    IF NEW.__created_by IS NOT NULL THEN
        -- v1 -> v2
        IF NEW.__logs IS NOT NULL THEN
            INSERT INTO job_logs (job_id, workspace_id, logs)
            VALUES (NEW.id, NEW.workspace_id, NEW.__logs)
            ON CONFLICT (job_id) DO UPDATE SET
                logs = CONCAT(job_logs.logs, EXCLUDED.logs)
            ;
            -- Need to be "before" to remove logs:
            NEW.__logs := NULL;
        END IF;
        RETURN NEW;
    END IF;
    -- v2 -> v1
    -- When inserting to `v2_job_queue` from `v2` code, set `v1` columns:
    SELECT * INTO job FROM v2_job WHERE id = NEW.id;
    NEW.__parent_job := job.parent_job;
    NEW.__created_by := job.created_by;
    NEW.__script_hash := job.runnable_id;
    NEW.__script_path := job.runnable_path;
    NEW.__args := job.args;
    -- __logs
    NEW.__raw_code := job.raw_code;
    NEW.__canceled := NEW.canceled_by IS NOT NULL;
    -- __last_ping
    NEW.__job_kind := job.kind;
    NEW.__env_id := 123456789; -- Magic used bellow.
    NEW.__schedule_path := CASE WHEN job.trigger_kind = 'schedule'::job_trigger_kind THEN job.trigger END;
    NEW.__permissioned_as := job.permissioned_as;
    -- __flow_status
    NEW.__raw_flow := job.raw_flow;
    NEW.__is_flow_step := job.flow_step_id IS NOT NULL;
    NEW.__language := job.script_lang;
    NEW.__same_worker := job.same_worker;
    NEW.__raw_lock := job.raw_lock;
    NEW.__pre_run_error := job.pre_run_error;
    NEW.__email := job.permissioned_as_email;
    NEW.__visible_to_owner := job.visible_to_owner;
    -- __mem_peak
    NEW.__root_job := job.flow_innermost_root_job;
    -- __leaf_jobs
    NEW.__concurrent_limit := job.concurrent_limit;
    NEW.__concurrency_time_window_s := job.concurrency_time_window_s;
    NEW.__timeout := job.timeout;
    NEW.__flow_step_id := job.flow_step_id;
    NEW.__cache_ttl := job.cache_ttl;
    IF job.script_entrypoint_override IS NOT NULL THEN
        NEW.__args = jsonb_set(
            coalesce(NEW.__args, '{}'::JSONB),
            '{_ENTRYPOINT_OVERRIDE}',
            to_jsonb(job.script_entrypoint_override)
        );
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_queue_before_insert_trigger
    BEFORE INSERT ON v2_job_queue
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_queue_before_insert();

-- v1 -> v2
-- On every insert to `v2_job_queue`, insert to `v2_job`, `v2_job_runtime` and `v2_job_status` as well
-- This trigger will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_queue_after_insert() RETURNS TRIGGER AS $$ BEGIN
    INSERT INTO v2_job (
        id, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email,
        kind, runnable_id, runnable_path, parent_job,
        script_lang, script_entrypoint_override,
        flow_step, flow_step_id, flow_innermost_root_job,
        trigger, trigger_kind,
        tag, same_worker, visible_to_owner, concurrent_limit, concurrency_time_window_s, cache_ttl, timeout, priority,
        preprocessed, args, pre_run_error,
        raw_code, raw_lock, raw_flow
    ) VALUES (
        NEW.id, NEW.workspace_id, NEW.created_at, NEW.__created_by, NEW.__permissioned_as, NEW.__email,
        NEW.__job_kind, NEW.__script_hash, NEW.__script_path, NEW.__parent_job,
        NEW.__language, NULLIF(NEW.__args->>'_ENTRYPOINT_OVERRIDE', '__WM_PREPROCESSOR'),
        NULL, NEW.__flow_step_id, NEW.__root_job,
        NEW.__schedule_path, CASE WHEN NEW.__schedule_path IS NOT NULL THEN 'schedule'::job_trigger_kind END,
        NEW.tag, NEW.__same_worker, NEW.__visible_to_owner, NEW.__concurrent_limit, NEW.__concurrency_time_window_s,
        NEW.__cache_ttl, NEW.__timeout, NEW.priority,
        CASE WHEN (
            NEW.__args->>'_ENTRYPOINT_OVERRIDE' IN ('__WM_PREPROCESSOR', 'preprocessor')
            OR NEW.__flow_status->'preprocessor_module' IS NOT NULL
        ) THEN FALSE END, NEW.__args, NEW.__pre_run_error,
        NEW.__raw_code, NEW.__raw_lock, NEW.__raw_flow
    ) ON CONFLICT (id) DO UPDATE SET
        workspace_id = EXCLUDED.workspace_id,
        created_at = EXCLUDED.created_at,
        created_by = EXCLUDED.created_by,
        permissioned_as = EXCLUDED.permissioned_as,
        permissioned_as_email = EXCLUDED.permissioned_as_email,
        kind = EXCLUDED.kind,
        runnable_id = EXCLUDED.runnable_id,
        runnable_path = EXCLUDED.runnable_path,
        parent_job = EXCLUDED.parent_job,
        script_lang = EXCLUDED.script_lang,
        script_entrypoint_override = EXCLUDED.script_entrypoint_override,
        flow_step = EXCLUDED.flow_step,
        flow_step_id = EXCLUDED.flow_step_id,
        flow_innermost_root_job = EXCLUDED.flow_innermost_root_job,
        trigger = EXCLUDED.trigger,
        trigger_kind = EXCLUDED.trigger_kind,
        tag = EXCLUDED.tag,
        same_worker = EXCLUDED.same_worker,
        visible_to_owner = EXCLUDED.visible_to_owner,
        concurrent_limit = EXCLUDED.concurrent_limit,
        concurrency_time_window_s = EXCLUDED.concurrency_time_window_s,
        cache_ttl = EXCLUDED.cache_ttl,
        timeout = EXCLUDED.timeout,
        priority = EXCLUDED.priority,
        preprocessed = EXCLUDED.preprocessed,
        args = EXCLUDED.args,
        pre_run_error = EXCLUDED.pre_run_error,
        raw_code = COALESCE(v2_job.raw_code, EXCLUDED.raw_code),
        raw_lock = COALESCE(v2_job.raw_lock, EXCLUDED.raw_lock),
        raw_flow = COALESCE(v2_job.raw_flow, EXCLUDED.raw_flow)
    ;
    INSERT INTO v2_job_runtime (id, ping, memory_peak)
    VALUES (NEW.id, NEW.__last_ping, NEW.__mem_peak)
    ON CONFLICT (id) DO UPDATE SET
        ping = COALESCE(v2_job_runtime.ping, EXCLUDED.ping),
        memory_peak = COALESCE(v2_job_runtime.memory_peak, EXCLUDED.memory_peak)
    ;
    IF NEW.__flow_status IS NOT NULL OR NEW.__leaf_jobs IS NOT NULL THEN
        INSERT INTO v2_job_status (id, flow_status, flow_leaf_jobs)
        VALUES (NEW.id, NEW.__flow_status, NEW.__leaf_jobs)
        ON CONFLICT (id) DO UPDATE SET
            flow_status = COALESCE(v2_job_status.flow_status, EXCLUDED.flow_status),
            flow_leaf_jobs = COALESCE(v2_job_status.flow_leaf_jobs, EXCLUDED.flow_leaf_jobs)
        ;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_queue_after_insert_trigger
    AFTER INSERT ON v2_job_queue
    FOR EACH ROW
    -- Prevent infinite loop v1 <-> v2
    WHEN (pg_trigger_depth() < 1 AND NEW.__created_by IS NOT NULL AND NEW.__env_id IS DISTINCT FROM 123456789)
EXECUTE FUNCTION v2_job_queue_after_insert();

-- On every update to `v2_job_queue`, update `v2_job`, `v2_job_runtime` and `v2_job_status` as well
-- This trigger will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_queue_before_update() RETURNS TRIGGER AS $$ BEGIN
    IF NEW.canceled_by IS NOT NULL THEN
        NEW.__canceled := TRUE;
    END IF;
    -- `v2_job`: Only `args` are updated
    IF NEW.__args::TEXT IS DISTINCT FROM OLD.__args::TEXT THEN
        UPDATE v2_job SET
            args = NEW.__args,
            preprocessed = CASE WHEN preprocessed = FALSE THEN TRUE ELSE preprocessed END
        WHERE id = NEW.id;
    END IF;
    -- `v2_job_runtime`:
    IF NEW.__last_ping IS DISTINCT FROM OLD.__last_ping OR NEW.__mem_peak IS DISTINCT FROM OLD.__mem_peak THEN
        INSERT INTO v2_job_runtime (id, ping, memory_peak)
        VALUES (NEW.id, NEW.__last_ping, NEW.__mem_peak)
        ON CONFLICT (id) DO UPDATE SET
            ping = EXCLUDED.ping,
            memory_peak = EXCLUDED.memory_peak
        ;
    END IF;
    -- `v2_job_status`:
    IF NEW.__flow_status::TEXT IS DISTINCT FROM OLD.__flow_status::TEXT OR
       NEW.__leaf_jobs::TEXT IS DISTINCT FROM OLD.__leaf_jobs::TEXT THEN
        IF NEW.__is_flow_step = false AND NEW.__parent_job IS NOT NULL THEN
            -- `workflow_as_code`:
            INSERT INTO v2_job_status (id, workflow_as_code_status)
            VALUES (NEW.id, NEW.__flow_status)
            ON CONFLICT (id) DO UPDATE SET
                workflow_as_code_status = EXCLUDED.workflow_as_code_status
            ;
        ELSE
            INSERT INTO v2_job_status (id, flow_status, flow_leaf_jobs)
            VALUES (NEW.id, NEW.__flow_status, NEW.__leaf_jobs)
            ON CONFLICT (id) DO UPDATE SET
                flow_status = EXCLUDED.flow_status,
                flow_leaf_jobs = EXCLUDED.flow_leaf_jobs
            ;
        END IF;
    END IF;
    -- `job_logs`:
    IF NEW.__logs IS DISTINCT FROM OLD.__logs THEN
        INSERT INTO job_logs (job_id, workspace_id, logs)
        VALUES (NEW.id, NEW.workspace_id, NEW.__logs)
        ON CONFLICT (job_id) DO UPDATE SET
            logs = CONCAT(job_logs.logs, EXCLUDED.logs)
        ;
        NEW.__logs := NULL;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_queue_before_update_trigger
    BEFORE UPDATE ON v2_job_queue
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_queue_before_update();

-- v2 -> v1: update preprocessed args
CREATE OR REPLACE FUNCTION v2_job_after_update() RETURNS TRIGGER AS $$ BEGIN
    UPDATE v2_job_queue SET __args = NEW.args WHERE id = NEW.id;
    UPDATE v2_job_completed SET __args = NEW.args WHERE id = NEW.id;
    RETURN NEW;
END $$ LANGUAGE PLPGSQL;

CREATE OR REPLACE TRIGGER v2_job_after_update_trigger
    AFTER UPDATE ON v2_job
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1 AND NEW.args::TEXT IS DISTINCT FROM OLD.args::TEXT) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_after_update();
