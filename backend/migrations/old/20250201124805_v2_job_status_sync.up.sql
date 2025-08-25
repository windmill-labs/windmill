-- Add up migration script here

-- On every insert/update to `v2_job_status`, reflect to `v2_job_queue` as well
-- This triggers will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_status_before_insert() RETURNS TRIGGER AS $$ BEGIN
    UPDATE v2_job_queue
    SET __flow_status = NEW.flow_status, __leaf_jobs = NEW.flow_leaf_jobs
    WHERE id = NEW.id;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_status_before_insert_trigger
    BEFORE INSERT ON v2_job_status
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_status_before_insert();

CREATE OR REPLACE FUNCTION v2_job_status_before_update() RETURNS TRIGGER AS $$ BEGIN
    IF NEW.flow_status::TEXT IS DISTINCT FROM OLD.flow_status::TEXT OR
       NEW.flow_leaf_jobs::TEXT IS DISTINCT FROM OLD.flow_leaf_jobs::TEXT THEN
        UPDATE v2_job_queue
        SET __flow_status = NEW.flow_status, __leaf_jobs = NEW.flow_leaf_jobs
        WHERE id = NEW.id;
    END IF;
    IF NEW.workflow_as_code_status::TEXT IS DISTINCT FROM OLD.workflow_as_code_status::TEXT THEN
        UPDATE v2_job_queue
        SET __flow_status = NEW.workflow_as_code_status
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_status_before_update_trigger
    BEFORE UPDATE ON v2_job_status
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_status_before_update();
