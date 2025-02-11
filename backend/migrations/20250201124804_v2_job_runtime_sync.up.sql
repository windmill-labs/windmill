-- Add up migration script here

-- On every insert/update to `v2_job_runtime`, reflect to `v2_job_queue` as well
-- This triggers will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_runtime_before_insert() RETURNS TRIGGER AS $$ BEGIN
    UPDATE v2_job_queue
    SET __last_ping = NEW.ping, __mem_peak = NEW.memory_peak
    WHERE id = NEW.id;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_runtime_before_insert_trigger
    BEFORE INSERT ON v2_job_runtime
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_runtime_before_insert();

CREATE OR REPLACE FUNCTION v2_job_runtime_before_update() RETURNS TRIGGER AS $$ BEGIN
    IF NEW.ping IS DISTINCT FROM OLD.ping OR NEW.memory_peak IS DISTINCT FROM OLD.memory_peak THEN
        UPDATE v2_job_queue
        SET __last_ping = NEW.ping, __mem_peak = NEW.memory_peak
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_runtime_before_update_trigger
    BEFORE UPDATE ON v2_job_runtime
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v2 <-> v1
EXECUTE FUNCTION v2_job_runtime_before_update();
