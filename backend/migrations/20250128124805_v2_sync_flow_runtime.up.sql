-- Add up migration script here

-- On every insert/update to `v2_job_flow_runtime`, reflect to `v2_job_queue` as well
-- This triggers will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_flow_runtime_before_insert() RETURNS TRIGGER AS $$ BEGIN
    UPDATE v2_job_queue
    SET __flow_status = NEW.flow_status, __leaf_jobs = NEW.leaf_jobs
    WHERE id = NEW.id;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_flow_runtime_before_insert_trigger
    BEFORE INSERT ON v2_job_flow_runtime
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_flow_runtime_before_insert();

CREATE OR REPLACE FUNCTION v2_job_flow_runtime_before_update() RETURNS TRIGGER AS $$ BEGIN
    IF NEW.flow_status::TEXT IS DISTINCT FROM OLD.flow_status::TEXT OR
       NEW.leaf_jobs::TEXT IS DISTINCT FROM OLD.leaf_jobs::TEXT THEN
        UPDATE v2_job_queue
        SET __flow_status = NEW.flow_status, __leaf_jobs = NEW.leaf_jobs
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_flow_runtime_before_update_trigger
    BEFORE UPDATE ON v2_job_flow_runtime
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_flow_runtime_before_update();
