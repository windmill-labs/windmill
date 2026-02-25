-- Fixture for preserve_on_behalf_of integration tests
-- Extends base.sql with a deployer user in the wm_deployers group

-- Include all base setup (workspace, admin user, etc.)
INSERT INTO workspace
            (id,               name,             owner)
     VALUES ('test-workspace', 'test-workspace', 'test-user')
ON CONFLICT DO NOTHING;

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace')
ON CONFLICT DO NOTHING;

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'all', 'All users', '{}')
ON CONFLICT DO NOTHING;

-- Create the wm_deployers group
INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'wm_deployers', 'Users allowed to deploy and preserve on_behalf_of', '{}')
ON CONFLICT DO NOTHING;

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('test@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Test User', 'test-user')
ON CONFLICT DO NOTHING;

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('test2@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 2')
ON CONFLICT DO NOTHING;

-- Deployer user (non-admin but in wm_deployers group)
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('deployer@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Deployer User')
ON CONFLICT DO NOTHING;

-- Original user whose on_behalf_of should be preserved
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('original@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Original User')
ON CONFLICT DO NOTHING;

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test2@windmill.dev', 'test-user-2', false, 'User')
ON CONFLICT DO NOTHING;

-- Deployer user in workspace
INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'deployer@windmill.dev', 'deployer-user', false, 'User')
ON CONFLICT DO NOTHING;

-- Original user in workspace (whose on_behalf_of should be preserved)
INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'original@windmill.dev', 'original-user', false, 'User')
ON CONFLICT DO NOTHING;

-- Add deployer user to wm_deployers group
INSERT INTO usr_to_group(workspace_id, group_, usr) VALUES
	('test-workspace', 'wm_deployers', 'deployer-user')
ON CONFLICT DO NOTHING;

-- Tokens for all users
INSERT INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true)
ON CONFLICT DO NOTHING;
INSERT INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN_2', 'test2@windmill.dev', 'test token 2', false)
ON CONFLICT DO NOTHING;
INSERT INTO token(token, email, label, super_admin) VALUES ('DEPLOYER_TOKEN', 'deployer@windmill.dev', 'deployer token', false)
ON CONFLICT DO NOTHING;
INSERT INTO token(token, email, label, super_admin) VALUES ('ORIGINAL_TOKEN', 'original@windmill.dev', 'original token', false)
ON CONFLICT DO NOTHING;

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;

CREATE OR REPLACE FUNCTION "notify_insert_on_completed_job" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('completed', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

DROP TRIGGER IF EXISTS "notify_insert_on_completed_job" ON "v2_job_completed";
CREATE TRIGGER "notify_insert_on_completed_job"
 AFTER INSERT ON "v2_job_completed"
    FOR EACH ROW
EXECUTE FUNCTION "notify_insert_on_completed_job" ();

CREATE OR REPLACE FUNCTION "notify_queue" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('queued', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

DROP TRIGGER IF EXISTS "notify_queue_after_insert" ON "v2_job_queue";
CREATE TRIGGER "notify_queue_after_insert"
 AFTER INSERT ON "v2_job_queue"
    FOR EACH ROW
EXECUTE FUNCTION "notify_queue" ();

DROP TRIGGER IF EXISTS "notify_queue_after_flow_status_update" ON "v2_job_status";
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
