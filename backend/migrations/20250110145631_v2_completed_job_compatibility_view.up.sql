-- Add up migration script here
CREATE OR REPLACE VIEW v2_completed_job AS
SELECT
    j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    c.duration_ms,
    c.status = 'success'       AS success,
    j.runnable_id              AS script_hash,
    j.runnable_path            AS script_path,
    j.args,
    c.result,
    c.deleted,
    j.raw_code,
    c.status = 'canceled'      AS canceled,
    c.canceled_by,
    c.canceled_reason,
    j.kind                     AS job_kind,
    j.schedule_path,
    j.permissioned_as,
    c.flow_status,
    j.raw_flow,
    j.flow_step_id IS NOT NULL AS is_flow_step,
    j.script_lang              AS language,
    c.started_at,
    c.status = 'skipped'       AS is_skipped,
    j.raw_lock,
    j.permissioned_as_email    AS email,
    j.visible_to_owner,
    c.memory_peak              AS mem_peak,
    j.tag,
    j.priority,
    NULL::TEXT                 AS logs,
    NULL::BIGINT               AS env_id
FROM v2_job_completed c
     JOIN v2_job j USING (id)
;

CREATE OR REPLACE FUNCTION v2_completed_job_update(OLD v2_completed_job, NEW v2_completed_job) RETURNS VOID AS $$ BEGIN
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
        OR NEW.raw_lock IS DISTINCT FROM OLD.raw_lock
        OR NEW.email IS DISTINCT FROM OLD.email
        OR NEW.visible_to_owner IS DISTINCT FROM OLD.visible_to_owner
        OR NEW.priority IS DISTINCT FROM OLD.priority
    THEN
        RAISE EXCEPTION 'Updating an immutable column in `v2_completed_job`';
    END IF;
    -- Update the `v2_job` table
    IF NEW.args::TEXT IS DISTINCT FROM OLD.args::TEXT THEN
        UPDATE v2_job
        SET args = NEW.args
        WHERE id = OLD.id;
    END IF;
    -- Update the `v2_job_completed` table
    IF NEW.result::TEXT IS DISTINCT FROM OLD.result::TEXT
        OR NEW.deleted IS DISTINCT FROM OLD.deleted
    THEN
        UPDATE v2_job_completed
        SET result = NEW.result,
            deleted = NEW.deleted
        WHERE id = OLD.id;
    END IF;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION v2_completed_job_instead_of_update() RETURNS TRIGGER AS $$ BEGIN
    -- v1 -> v2 sync
    PERFORM v2_completed_job_update(OLD, NEW);
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION v2_completed_job_instead_of_update_overlay() RETURNS TRIGGER AS $$ BEGIN
    -- v1 -> v2 sync
    PERFORM v2_completed_job_update(OLD, NEW);
    -- v2 -> v1 sync
    IF NEW.args::TEXT IS DISTINCT FROM OLD.args::TEXT THEN
        UPDATE v2_job_completed
        SET __args = NEW.args
        WHERE id = OLD.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER v2_completed_job_instead_of_update_trigger
    INSTEAD OF UPDATE ON v2_completed_job
    FOR EACH ROW
EXECUTE PROCEDURE v2_completed_job_instead_of_update_overlay();

CREATE OR REPLACE FUNCTION v2_completed_job_instead_of_delete() RETURNS TRIGGER AS $$ BEGIN
    DELETE FROM v2_job_completed WHERE id = OLD.id;
    RETURN OLD;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER v2_completed_job_instead_of_delete_trigger
    INSTEAD OF DELETE ON v2_completed_job
    FOR EACH ROW
EXECUTE PROCEDURE v2_completed_job_instead_of_delete();
