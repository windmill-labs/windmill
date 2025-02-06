-- Add down migration script here
ALTER TABLE completed_job RENAME TO v2_job_completed;
CREATE OR REPLACE VIEW completed_job AS (
    SELECT
        id,
        workspace_id,
        parent_job AS parent_job,
        created_by AS created_by,
        created_at AS created_at,
        duration_ms,
        success AS success,
        script_hash AS script_hash,
        script_path AS script_path,
        args AS args,
        result,
        logs AS logs,
        deleted,
        raw_code AS raw_code,
        canceled AS canceled,
        canceled_by,
        canceled_reason,
        job_kind AS job_kind,
        env_id AS env_id,
        schedule_path AS schedule_path,
        permissioned_as AS permissioned_as,
        flow_status,
        raw_flow AS raw_flow,
        is_flow_step AS is_flow_step,
        language AS language,
        started_at,
        is_skipped AS is_skipped,
        raw_lock AS raw_lock,
        email AS email,
        visible_to_owner AS visible_to_owner,
        mem_peak AS mem_peak,
        tag AS tag,
        priority AS priority
    FROM v2_job_completed
);
