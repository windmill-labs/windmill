-- Add up migration script here
CREATE OR REPLACE VIEW v2_queue AS
SELECT
    j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    q.started_at,
    q.scheduled_for,
    q.running,
    j.runnable_id              AS script_hash,
    j.runnable_path            AS script_path,
    j.args,
    j.raw_code,
    q.canceled_by IS NOT NULL  AS canceled,
    q.canceled_by,
    q.canceled_reason,
    r.ping                     AS last_ping,
    j.kind                     AS job_kind,
    j.schedule_path,
    j.permissioned_as,
    f.flow_status,
    j.raw_flow,
    j.flow_step_id IS NOT NULL AS is_flow_step,
    j.script_lang              AS language,
    q.suspend,
    q.suspend_until,
    j.same_worker,
    j.raw_lock,
    j.pre_run_error,
    j.permissioned_as_email    AS email,
    j.visible_to_owner,
    r.memory_peak              AS mem_peak,
    j.flow_root_job            AS root_job,
    f.leaf_jobs,
    j.tag,
    j.concurrent_limit,
    j.concurrency_time_window_s,
    j.timeout,
    j.flow_step_id,
    j.cache_ttl,
    j.priority,
    NULL::TEXT                 AS logs
FROM v2_job_queue q
     JOIN v2_job j USING (id)
     LEFT JOIN v2_job_runtime r USING (id)
     LEFT JOIN v2_job_flow_runtime f USING (id)
;

-- Dispatch update of `v1` schema to `v2_*` tables.
CREATE OR REPLACE FUNCTION v2_queue_update(OLD v2_queue, NEW v2_queue) RETURNS VOID AS $$ BEGIN
    -- Unsupported columns:
    IF NEW.workspace_id IS DISTINCT FROM OLD.workspace_id
        OR NEW.parent_job IS DISTINCT FROM OLD.parent_job
        OR NEW.created_by IS DISTINCT FROM OLD.created_by
        OR NEW.created_at IS DISTINCT FROM OLD.created_at
        OR NEW.script_hash IS DISTINCT FROM OLD.script_hash
        OR NEW.script_path IS DISTINCT FROM OLD.script_path
        OR NEW.raw_code IS DISTINCT FROM OLD.raw_code
        OR NEW.job_kind IS DISTINCT FROM OLD.job_kind
        OR NEW.schedule_path IS DISTINCT FROM OLD.schedule_path
        OR NEW.permissioned_as IS DISTINCT FROM OLD.permissioned_as
        OR NEW.raw_flow::TEXT IS DISTINCT FROM OLD.raw_flow::TEXT
        OR NEW.language IS DISTINCT FROM OLD.language
        OR NEW.same_worker IS DISTINCT FROM OLD.same_worker
        OR NEW.raw_lock IS DISTINCT FROM OLD.raw_lock
        OR NEW.pre_run_error IS DISTINCT FROM OLD.pre_run_error
        OR NEW.email IS DISTINCT FROM OLD.email
        OR NEW.visible_to_owner IS DISTINCT FROM OLD.visible_to_owner
        OR NEW.concurrent_limit IS DISTINCT FROM OLD.concurrent_limit
        OR NEW.concurrency_time_window_s IS DISTINCT FROM OLD.concurrency_time_window_s
        OR NEW.timeout IS DISTINCT FROM OLD.timeout
        OR NEW.flow_step_id IS DISTINCT FROM OLD.flow_step_id
        OR NEW.cache_ttl IS DISTINCT FROM OLD.cache_ttl
        OR NEW.priority IS DISTINCT FROM OLD.priority
    THEN
        RAISE EXCEPTION 'Updating an immutable column in `v2_queue`';
    END IF;
    -- Update the `v2_job` table
    IF NEW.args::TEXT IS DISTINCT FROM OLD.args::TEXT THEN
        UPDATE v2_job
        SET args = NEW.args
        WHERE id = OLD.id;
    END IF;
    -- Update the `v2_job_queue` table
    IF NEW.canceled AND NEW.canceled_by IS NULL THEN
        NEW.canceled_by := 'unknown';
        NEW.canceled_reason := 'canceled by user';
    END IF;
    IF NEW.started_at IS DISTINCT FROM OLD.started_at
        OR NEW.scheduled_for IS DISTINCT FROM OLD.scheduled_for
        OR NEW.running IS DISTINCT FROM OLD.running
        OR NEW.canceled_by IS DISTINCT FROM OLD.canceled_by
        OR NEW.canceled_reason IS DISTINCT FROM OLD.canceled_reason
        OR NEW.suspend IS DISTINCT FROM OLD.suspend
        OR NEW.suspend_until IS DISTINCT FROM OLD.suspend_until
    THEN
        UPDATE v2_job_queue
        SET started_at      = NEW.started_at,
            scheduled_for   = NEW.scheduled_for,
            running         = NEW.running,
            canceled_by     = NEW.canceled_by,
            canceled_reason = NEW.canceled_reason,
            suspend         = NEW.suspend,
            suspend_until   = NEW.suspend_until
        WHERE id = OLD.id;
    END IF;
    -- Update the `v2_job_runtime` table
    IF NEW.last_ping IS DISTINCT FROM OLD.last_ping
        OR NEW.mem_peak IS DISTINCT FROM OLD.mem_peak
    THEN
        UPDATE v2_job_runtime
        SET ping        = NEW.last_ping,
            memory_peak = NEW.mem_peak
        WHERE id = OLD.id;
    END IF;
    -- Update the `v2_job_flow_runtime` table
    IF NEW.flow_status::TEXT IS DISTINCT FROM OLD.flow_status::TEXT
        OR NEW.leaf_jobs::TEXT IS DISTINCT FROM OLD.leaf_jobs::TEXT
    THEN
        UPDATE v2_job_flow_runtime
        SET flow_status = NEW.flow_status,
            leaf_jobs   = NEW.leaf_jobs
        WHERE id = OLD.id;
    END IF;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION v2_queue_instead_of_update() RETURNS TRIGGER AS $$ BEGIN
    -- v1 -> v2 sync
    PERFORM v2_queue_update(OLD, NEW);
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION v2_queue_instead_of_update_overlay() RETURNS TRIGGER AS $$ BEGIN
    -- v1 -> v2 sync
    PERFORM v2_queue_update(OLD, NEW);
    -- v2 -> v1 sync
    IF NEW.args IS DISTINCT FROM OLD.args
        OR NEW.last_ping IS DISTINCT FROM OLD.last_ping
        OR NEW.mem_peak IS DISTINCT FROM OLD.mem_peak
        OR NEW.flow_status::TEXT IS DISTINCT FROM OLD.flow_status::TEXT
        OR NEW.leaf_jobs::TEXT IS DISTINCT FROM OLD.leaf_jobs::TEXT
    THEN
        UPDATE v2_job_queue
        SET __args          = NEW.args,
            __last_ping     = NEW.last_ping,
            __mem_peak      = NEW.mem_peak,
            __flow_status   = NEW.flow_status,
            __leaf_jobs     = NEW.leaf_jobs
        WHERE id = OLD.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER v2_queue_instead_of_update_trigger
INSTEAD OF UPDATE ON v2_queue
FOR EACH ROW
EXECUTE PROCEDURE v2_queue_instead_of_update_overlay();

CREATE OR REPLACE FUNCTION v2_queue_instead_of_delete() RETURNS TRIGGER AS $$ BEGIN
    DELETE FROM v2_job_queue WHERE id = OLD.id;
    RETURN OLD;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER v2_queue_instead_of_delete_trigger
    INSTEAD OF DELETE ON v2_queue
    FOR EACH ROW
EXECUTE PROCEDURE v2_queue_instead_of_delete();
