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

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('test@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Test User', 'test-user');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('test2@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 2');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('test3@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 3');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test2@windmill.dev', 'test-user-2', false, 'User');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test3@windmill.dev', 'test-user-3', false, 'User');

insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true);
insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN_2', 'test2@windmill.dev', 'test token 2', false);
insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN_3', 'test3@windmill.dev', 'test token 3', false);

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

-- Apply phase 4:
DROP FUNCTION IF EXISTS v2_job_after_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_completed_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_completed_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_after_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_runtime_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_runtime_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_status_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_status_before_update CASCADE;

DROP VIEW IF EXISTS completed_job, completed_job_view, job, queue, queue_view CASCADE;

ALTER TABLE v2_job_queue
    DROP COLUMN IF EXISTS __parent_job CASCADE,
    DROP COLUMN IF EXISTS __created_by CASCADE,
    DROP COLUMN IF EXISTS __script_hash CASCADE,
    DROP COLUMN IF EXISTS __script_path CASCADE,
    DROP COLUMN IF EXISTS __args CASCADE,
    DROP COLUMN IF EXISTS __logs CASCADE,
    DROP COLUMN IF EXISTS __raw_code CASCADE,
    DROP COLUMN IF EXISTS __canceled CASCADE,
    DROP COLUMN IF EXISTS __last_ping CASCADE,
    DROP COLUMN IF EXISTS __job_kind CASCADE,
    DROP COLUMN IF EXISTS __env_id CASCADE,
    DROP COLUMN IF EXISTS __schedule_path CASCADE,
    DROP COLUMN IF EXISTS __permissioned_as CASCADE,
    DROP COLUMN IF EXISTS __flow_status CASCADE,
    DROP COLUMN IF EXISTS __raw_flow CASCADE,
    DROP COLUMN IF EXISTS __is_flow_step CASCADE,
    DROP COLUMN IF EXISTS __language CASCADE,
    DROP COLUMN IF EXISTS __same_worker CASCADE,
    DROP COLUMN IF EXISTS __raw_lock CASCADE,
    DROP COLUMN IF EXISTS __pre_run_error CASCADE,
    DROP COLUMN IF EXISTS __email CASCADE,
    DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
    DROP COLUMN IF EXISTS __mem_peak CASCADE,
    DROP COLUMN IF EXISTS __root_job CASCADE,
    DROP COLUMN IF EXISTS __leaf_jobs CASCADE,
    DROP COLUMN IF EXISTS __concurrent_limit CASCADE,
    DROP COLUMN IF EXISTS __concurrency_time_window_s CASCADE,
    DROP COLUMN IF EXISTS __timeout CASCADE,
    DROP COLUMN IF EXISTS __flow_step_id CASCADE,
    DROP COLUMN IF EXISTS __cache_ttl CASCADE;

LOCK TABLE v2_job_queue IN ACCESS EXCLUSIVE MODE;
ALTER TABLE v2_job_completed
    DROP COLUMN IF EXISTS __parent_job CASCADE,
    DROP COLUMN IF EXISTS __created_by CASCADE,
    DROP COLUMN IF EXISTS __created_at CASCADE,
    DROP COLUMN IF EXISTS __success CASCADE,
    DROP COLUMN IF EXISTS __script_hash CASCADE,
    DROP COLUMN IF EXISTS __script_path CASCADE,
    DROP COLUMN IF EXISTS __args CASCADE,
    DROP COLUMN IF EXISTS __logs CASCADE,
    DROP COLUMN IF EXISTS __raw_code CASCADE,
    DROP COLUMN IF EXISTS __canceled CASCADE,
    DROP COLUMN IF EXISTS __job_kind CASCADE,
    DROP COLUMN IF EXISTS __env_id CASCADE,
    DROP COLUMN IF EXISTS __schedule_path CASCADE,
    DROP COLUMN IF EXISTS __permissioned_as CASCADE,
    DROP COLUMN IF EXISTS __raw_flow CASCADE,
    DROP COLUMN IF EXISTS __is_flow_step CASCADE,
    DROP COLUMN IF EXISTS __language CASCADE,
    DROP COLUMN IF EXISTS __is_skipped CASCADE,
    DROP COLUMN IF EXISTS __raw_lock CASCADE,
    DROP COLUMN IF EXISTS __email CASCADE,
    DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
    DROP COLUMN IF EXISTS __tag CASCADE,
    DROP COLUMN IF EXISTS __priority CASCADE;
