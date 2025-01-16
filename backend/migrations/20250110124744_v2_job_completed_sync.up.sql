-- Add up migration script here

-- v1 -> v2
-- On every insert to `v2_job_completed`, insert to `v2_job` as well
-- This trigger will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_completed_before_insert()
RETURNS TRIGGER AS $$
DECLARE
    job v2_job;
BEGIN
    IF NEW.__created_by IS NULL THEN
        -- v2 -> v1
        SELECT * INTO job FROM v2_job WHERE id = NEW.id;
        NEW.__parent_job := job.parent_job;
        NEW.__created_by := job.created_by;
        NEW.__created_at := job.created_at;
        NEW.__success := NEW.status = 'success';
        NEW.__script_hash := job.runnable_id;
        NEW.__script_path := job.runnable_path;
        NEW.__args := job.args;
        -- __logs
        -- __deleted
        NEW.__raw_code := job.raw_code;
        NEW.__canceled := NEW.status = 'canceled';
        NEW.__job_kind := job.kind;
        -- __env_id
        NEW.__schedule_path := job.schedule_path;
        NEW.__permissioned_as := job.permissioned_as;
        NEW.__raw_flow := job.raw_flow;
        NEW.__is_flow_step := job.flow_step_id IS NOT NULL;
        NEW.__language := job.script_lang;
        NEW.__is_skipped := NEW.status = 'skipped';
        NEW.__raw_lock := job.raw_lock;
        NEW.__email := job.permissioned_as_email;
        NEW.__visible_to_owner := job.visible_to_owner;
        NEW.__tag := job.tag;
        NEW.__priority := job.priority;
    ELSE
        -- v1 -> v2
        NEW.completed_at := now();
        NEW.status := CASE
            WHEN NEW.__is_skipped THEN 'skipped'::job_status
            WHEN NEW.__canceled THEN 'canceled'::job_status
            WHEN NEW.__success THEN 'success'::job_status
            ELSE 'failure'::job_status
        END;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_completed_before_insert_trigger
BEFORE INSERT ON v2_job_completed
FOR EACH ROW
WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_completed_before_insert();
