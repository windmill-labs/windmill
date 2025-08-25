-- Add up migration script here

-- v1 -> v2
-- On every insert to `v2_job_completed`, insert to `v2_job` as well
-- This trigger will be removed once all server(s)/worker(s) are updated to use `v2_*` tables
CREATE OR REPLACE FUNCTION v2_job_completed_before_insert() RETURNS TRIGGER AS $$
DECLARE job v2_job;
DECLARE final_labels TEXT[];
BEGIN
    -- New columns synchronization:
    -- 1. `result_columns` <-> `flow_status._metadata.column_order`
    -- 2. `v2_job.labels` <-> `result.wm_labels`
    -- 3. v2 <-> flow_status._metadata.preprocessed_args`
    IF NEW.__created_by IS NULL THEN
        -- v2 -> v1
        -- When inserting to `v2_job_completed` from `v2` code, set `v1` columns:
        SELECT * INTO job FROM v2_job WHERE id = NEW.id;
        NEW.__parent_job := job.parent_job;
        NEW.__created_by := job.created_by;
        NEW.__created_at := job.created_at;
        NEW.__success := NEW.status = 'success'::job_status;
        NEW.__script_hash := job.runnable_id;
        NEW.__script_path := job.runnable_path;
        NEW.__args := job.args;
        -- __logs
        NEW.__raw_code := job.raw_code;
        NEW.__canceled := NEW.status = 'canceled'::job_status;
        NEW.__job_kind := job.kind;
        -- __env_id
        NEW.__schedule_path := CASE WHEN job.trigger_kind = 'schedule'::job_trigger_kind THEN job.trigger END;
        NEW.__permissioned_as := job.permissioned_as;
        NEW.__raw_flow := job.raw_flow;
        NEW.__is_flow_step := job.flow_step_id IS NOT NULL;
        NEW.__language := job.script_lang;
        NEW.__is_skipped := NEW.status = 'skipped'::job_status;
        NEW.__raw_lock := job.raw_lock;
        NEW.__email := job.permissioned_as_email;
        NEW.__visible_to_owner := job.visible_to_owner;
        NEW.__tag := job.tag;
        NEW.__priority := job.priority;
        -- 1. `result_columns` -> `flow_status._metadata.column_order`
        IF NEW.result_columns IS NOT NULL AND (NEW.flow_status IS NULL OR jsonb_typeof(NEW.flow_status) = 'object') THEN
            NEW.flow_status := jsonb_set(
                coalesce(NEW.flow_status, '{}'::JSONB),
                '{_metadata}',
                jsonb_set(
                    coalesce(NEW.flow_status->'_metadata', '{}'::JSONB),
                    '{column_order}',
                    to_jsonb(NEW.result_columns)
                )
            );
        END IF;
        -- 2. `v2_job.labels` -> `result.wm_labels`
        IF job.labels IS NOT NULL AND (NEW.result IS NULL OR jsonb_typeof(NEW.result) = 'object') THEN
            IF jsonb_typeof(NEW.result->'wm_labels') = 'array' AND (
                SELECT bool_and(jsonb_typeof(elem) = 'string')
                FROM jsonb_array_elements(NEW.result->'wm_labels') AS elem
            ) THEN
                SELECT array_agg(DISTINCT all_labels) INTO final_labels
                FROM unnest(
                    coalesce(job.labels, ARRAY[]::TEXT[]) || translate(NEW.result->>'wm_labels', '[]', '{}')::TEXT[]
                ) all_labels;
            ELSE
                final_labels := job.labels;
            END IF;
            -- Update `v2_job.labels` if needed
            IF job.labels IS DISTINCT FROM final_labels THEN
                UPDATE v2_job SET labels = final_labels WHERE id = NEW.id;
            END IF;
            NEW.result := jsonb_set(
                coalesce(NEW.result, '{}'::JSONB),
                '{wm_labels}',
                to_jsonb(final_labels)
            );
        END IF;
        -- 3. v2 -> flow_status._metadata.preprocessed_args`
        IF job.kind = 'script' AND job.preprocessed = TRUE
            AND (NEW.flow_status IS NULL OR jsonb_typeof(NEW.flow_status) = 'object')
        THEN
            NEW.flow_status := jsonb_set(
                coalesce(NEW.flow_status, '{}'::JSONB),
                '{_metadata}',
                jsonb_set(
                    coalesce(NEW.flow_status->'_metadata', '{}'::JSONB),
                    '{preprocessed_args}',
                    'true'::JSONB
                )
            );
        END IF;
    ELSE
        -- v1 -> v2
        NEW.completed_at := now();
        NEW.status := CASE
            WHEN NEW.__is_skipped THEN 'skipped'::job_status
            WHEN NEW.__canceled THEN 'canceled'::job_status
            WHEN NEW.__success THEN 'success'::job_status
            ELSE 'failure'::job_status
        END;
        -- 1. `result_columns` <- `flow_status._metadata.column_order`
        IF jsonb_typeof(NEW.flow_status->'_metadata'->'column_order') = 'array' AND (
            SELECT bool_and(jsonb_typeof(elem) = 'string')
            FROM jsonb_array_elements(NEW.flow_status->'_metadata'->'column_order') AS elem
        ) THEN
            NEW.result_columns := translate(NEW.flow_status->'_metadata'->>'column_order', '[]', '{}')::TEXT[];
        END IF;
        -- 2. `v2_job.labels` <- `result.wm_labels`
        IF jsonb_typeof(NEW.result->'wm_labels') = 'array' AND (
            SELECT bool_and(jsonb_typeof(elem) = 'string')
            FROM jsonb_array_elements(NEW.result->'wm_labels') AS elem
        ) THEN
            UPDATE v2_job SET
                labels = (
                    SELECT array_agg(DISTINCT all_labels)
                    FROM unnest(
                        coalesce(labels, ARRAY[]::TEXT[])
                            || translate(NEW.result->>'wm_labels', '[]', '{}')::TEXT[]
                    ) all_labels
                )
            WHERE id = NEW.id;
        END IF;
        -- 3. v2 <- flow_status._metadata.preprocessed_args`
        IF NEW.flow_status->'_metadata'->'preprocessed_args' = 'true'::JSONB THEN
            UPDATE v2_job SET
                args = NEW.__args,
                preprocessed = TRUE
            WHERE id = NEW.id AND preprocessed = FALSE;
        END IF;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_completed_before_insert_trigger
    BEFORE INSERT ON v2_job_completed
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_completed_before_insert();

CREATE OR REPLACE FUNCTION v2_job_completed_before_update() RETURNS TRIGGER AS $$ BEGIN
    -- `v2_job`: Only `args` are updated
    IF NEW.__args::TEXT IS DISTINCT FROM OLD.__args::TEXT THEN
        UPDATE v2_job SET
            args = NEW.__args,
            preprocessed = CASE WHEN preprocessed = FALSE THEN TRUE ELSE preprocessed END
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER v2_job_completed_before_update_trigger
    BEFORE UPDATE ON v2_job_completed
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 1) -- Prevent infinite loop v1 <-> v2
EXECUTE FUNCTION v2_job_completed_before_update();
