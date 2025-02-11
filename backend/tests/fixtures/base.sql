-- used for backend automated testing
-- https://docs.rs/sqlx/latest/sqlx/attr.test.html

INSERT INTO workspace
            (id,               name,             owner)
     VALUES ('test-workspace', 'test-workspace', 'test-user');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key');


INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace');

insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true);

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;

CREATE FUNCTION "notify_insert_on_completed_job" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('completed', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_insert_on_completed_job"
 AFTER INSERT ON "v2_job_completed"
    FOR EACH ROW
EXECUTE FUNCTION "notify_insert_on_completed_job" ();


CREATE FUNCTION "notify_queue" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('queued', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_queue_after_insert"
 AFTER INSERT ON "v2_job_queue"
    FOR EACH ROW
EXECUTE FUNCTION "notify_queue" ();

  CREATE TRIGGER "notify_queue_after_flow_status_update"
 AFTER UPDATE ON "v2_job_status"
    FOR EACH ROW
            WHEN (NEW.flow_status IS DISTINCT FROM OLD.flow_status)
EXECUTE FUNCTION "notify_queue" ();

-- TODO(uael): remove before phase 4
CREATE OR REPLACE FUNCTION zzz_v2_job_queue_integrity_check() RETURNS TRIGGER AS $$
DECLARE job v2_job;
DECLARE job_runtime v2_job_runtime;
DECLARE job_status v2_job_status;
BEGIN
    IF (OLD.canceled_by IS NOT NULL) IS DISTINCT FROM OLD.__canceled THEN
        RAISE EXCEPTION 'canceled mismatch';
    END IF;
    -- v2_job:
    SELECT * INTO job FROM v2_job WHERE id = OLD.id;
    IF job.tag IS DISTINCT FROM OLD.tag THEN
        RAISE EXCEPTION 'tag mismatch';
    END IF;
    IF job.workspace_id IS DISTINCT FROM OLD.workspace_id THEN
        RAISE EXCEPTION 'workspace_id mismatch';
    END IF;
    IF job.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'created_at mismatch';
    END IF;
    IF job.created_by IS DISTINCT FROM OLD.__created_by THEN
        RAISE EXCEPTION 'created_by mismatch';
    END IF;
    IF job.permissioned_as IS DISTINCT FROM OLD.__permissioned_as THEN
        RAISE EXCEPTION 'permissioned_as mismatch';
    END IF;
    IF job.permissioned_as_email IS DISTINCT FROM OLD.__email THEN
        RAISE EXCEPTION 'permissioned_as_email mismatch';
    END IF;
    IF job.kind IS DISTINCT FROM OLD.__job_kind THEN
        RAISE EXCEPTION 'kind mismatch';
    END IF;
    IF job.runnable_id IS DISTINCT FROM OLD.__script_hash THEN
        RAISE EXCEPTION 'runnable_id mismatch';
    END IF;
    IF job.runnable_path IS DISTINCT FROM OLD.__script_path THEN
        RAISE EXCEPTION 'runnable_path mismatch';
    END IF;
    IF job.parent_job IS DISTINCT FROM OLD.__parent_job THEN
        RAISE EXCEPTION 'parent_job mismatch';
    END IF;
    IF job.script_lang IS DISTINCT FROM OLD.__language THEN
        RAISE EXCEPTION 'script_lang mismatch';
    END IF;
    IF job.script_entrypoint_override IS DISTINCT FROM NULLIF(OLD.__args->>'_ENTRYPOINT_OVERRIDE', '__WM_PREPROCESSOR')
        AND OLD.__args->>'reason' IS DISTINCT FROM 'PREPROCESSOR_ARGS_ARE_DISCARDED'
    THEN
        RAISE EXCEPTION 'script_entrypoint_override mismatch';
    END IF;
    IF job.flow_step_id IS DISTINCT FROM OLD.__flow_step_id THEN
        RAISE EXCEPTION 'flow_step_id mismatch';
    END IF;
    IF (job.flow_step_id IS NOT NULL) IS DISTINCT FROM OLD.__is_flow_step THEN
        RAISE EXCEPTION 'is_flow_step mismatch';
    END IF;
    IF job.flow_innermost_root_job IS DISTINCT FROM OLD.__root_job THEN
        RAISE EXCEPTION 'flow_innermost_root_job mismatch';
    END IF;
    IF job.trigger IS DISTINCT FROM OLD.__schedule_path THEN
        RAISE EXCEPTION 'trigger mismatch';
    END IF;
    IF job.same_worker IS DISTINCT FROM OLD.__same_worker THEN
        RAISE EXCEPTION 'same_worker mismatch';
    END IF;
    IF job.visible_to_owner IS DISTINCT FROM OLD.__visible_to_owner THEN
        RAISE EXCEPTION 'visible_to_owner mismatch';
    END IF;
    IF job.concurrent_limit IS DISTINCT FROM OLD.__concurrent_limit THEN
        RAISE EXCEPTION 'concurrent_limit mismatch';
    END IF;
    IF job.concurrency_time_window_s IS DISTINCT FROM OLD.__concurrency_time_window_s THEN
        RAISE EXCEPTION 'concurrency_time_window_s mismatch';
    END IF;
    IF job.cache_ttl IS DISTINCT FROM OLD.__cache_ttl THEN
        RAISE EXCEPTION 'cache_ttl mismatch';
    END IF;
    IF job.timeout IS DISTINCT FROM OLD.__timeout THEN
        RAISE EXCEPTION 'timeout mismatch';
    END IF;
    IF job.priority IS DISTINCT FROM OLD.priority THEN
        RAISE EXCEPTION 'priority mismatch';
    END IF;
    IF job.args::TEXT IS DISTINCT FROM OLD.__args::TEXT AND OLD.__args->>'_ENTRYPOINT_OVERRIDE' IS DISTINCT FROM '__WM_PREPROCESSOR' THEN
        RAISE EXCEPTION 'args mismatch';
    END IF;
    IF job.pre_run_error IS DISTINCT FROM OLD.__pre_run_error THEN
        RAISE EXCEPTION 'pre_run_error mismatch';
    END IF;
    -- v2_job_runtime:
    SELECT * INTO job_runtime FROM v2_job_runtime WHERE id = OLD.id;
    IF job_runtime.ping IS DISTINCT FROM OLD.__last_ping THEN
        RAISE EXCEPTION 'ping mismatch';
    END IF;
    IF job_runtime.memory_peak IS DISTINCT FROM OLD.__mem_peak THEN
        RAISE EXCEPTION 'memory_peak mismatch';
    END IF;
    -- v2_job_status:
    IF EXISTS(SELECT 1 FROM v2_job_status  WHERE id = OLD.id) THEN
        SELECT * INTO job_status FROM v2_job_status WHERE id = OLD.id;
        IF COALESCE(job_status.flow_status, job_status.workflow_as_code_status)::TEXT IS DISTINCT FROM OLD.__flow_status::TEXT
        THEN
            RAISE EXCEPTION 'flow_status mismatch';
        END IF;
        IF job_status.flow_leaf_jobs::TEXT IS DISTINCT FROM OLD.__leaf_jobs::TEXT THEN
            RAISE EXCEPTION 'leaf_jobs mismatch';
        END IF;
    END IF;
    RETURN OLD;
END $$ LANGUAGE PLPGSQL;

CREATE OR REPLACE TRIGGER zzz_v2_job_queue_integrity_check_before_delete
    BEFORE DELETE ON v2_job_queue
    FOR EACH ROW
EXECUTE FUNCTION zzz_v2_job_queue_integrity_check();

-- TODO(uael): remove before phase 4
CREATE OR REPLACE FUNCTION zzz_v2_job_completed_integrity_check() RETURNS TRIGGER AS $$
DECLARE job v2_job;
BEGIN
    IF (NEW.canceled_by IS NOT NULL) IS DISTINCT FROM NEW.__canceled THEN
        RAISE EXCEPTION 'canceled mismatch';
    END IF;
    SELECT * INTO job FROM v2_job WHERE id = NEW.id;
    IF job.tag IS DISTINCT FROM NEW.__tag THEN
        RAISE EXCEPTION 'tag mismatch % %', job.tag, NEW.__tag;
    END IF;
    IF job.workspace_id IS DISTINCT FROM NEW.workspace_id THEN
        RAISE EXCEPTION 'workspace_id mismatch';
    END IF;
    IF job.created_at IS DISTINCT FROM NEW.__created_at THEN
        RAISE EXCEPTION 'created_at mismatch';
    END IF;
    IF job.created_by IS DISTINCT FROM NEW.__created_by THEN
        RAISE EXCEPTION 'created_by mismatch';
    END IF;
    IF job.permissioned_as IS DISTINCT FROM NEW.__permissioned_as THEN
        RAISE EXCEPTION 'permissioned_as mismatch';
    END IF;
    IF job.permissioned_as_email IS DISTINCT FROM NEW.__email THEN
        RAISE EXCEPTION 'permissioned_as_email mismatch';
    END IF;
    IF job.kind IS DISTINCT FROM NEW.__job_kind THEN
        RAISE EXCEPTION 'kind mismatch';
    END IF;
    IF job.runnable_id IS DISTINCT FROM NEW.__script_hash THEN
        RAISE EXCEPTION 'runnable_id mismatch';
    END IF;
    IF job.runnable_path IS DISTINCT FROM NEW.__script_path THEN
        RAISE EXCEPTION 'runnable_path mismatch';
    END IF;
    IF job.parent_job IS DISTINCT FROM NEW.__parent_job THEN
        RAISE EXCEPTION 'parent_job mismatch';
    END IF;
    IF job.script_lang IS DISTINCT FROM NEW.__language THEN
        RAISE EXCEPTION 'script_lang mismatch';
    END IF;
    IF job.script_entrypoint_override IS DISTINCT FROM NULLIF(NEW.__args->>'_ENTRYPOINT_OVERRIDE', '__WM_PREPROCESSOR')
        AND NEW.__args->>'reason' IS DISTINCT FROM 'PREPROCESSOR_ARGS_ARE_DISCARDED'
    THEN
        RAISE EXCEPTION 'script_entrypoint_override mismatch';
    END IF;
    IF (job.flow_step_id IS NOT NULL) IS DISTINCT FROM NEW.__is_flow_step THEN
        RAISE EXCEPTION 'is_flow_step mismatch';
    END IF;
    IF job.trigger IS DISTINCT FROM NEW.__schedule_path THEN
        RAISE EXCEPTION 'trigger mismatch';
    END IF;
    IF job.visible_to_owner IS DISTINCT FROM NEW.__visible_to_owner THEN
        RAISE EXCEPTION 'visible_to_owner mismatch';
    END IF;
    IF job.args::TEXT IS DISTINCT FROM NEW.__args::TEXT AND NEW.__args->>'_ENTRYPOINT_OVERRIDE' IS DISTINCT FROM '__WM_PREPROCESSOR' THEN
        RAISE EXCEPTION 'args mismatch';
    END IF;
    RETURN NEW;
END $$ LANGUAGE PLPGSQL;

CREATE OR REPLACE TRIGGER zzz_v2_job_completed_integrity_check_after_insert
    AFTER INSERT ON v2_job_completed
    FOR EACH ROW
EXECUTE FUNCTION zzz_v2_job_completed_integrity_check();
