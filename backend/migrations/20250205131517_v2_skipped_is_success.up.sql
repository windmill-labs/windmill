-- Add up migration script here
CREATE OR REPLACE VIEW v2_as_completed_job AS
SELECT
    j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    c.duration_ms,
    c.status = 'success' OR c.status = 'skipped' AS success,
    j.runnable_id              AS script_hash,
    j.runnable_path            AS script_path,
    j.args,
    c.result,
    FALSE                      AS deleted,
    j.raw_code,
    c.status = 'canceled'      AS canceled,
    c.canceled_by,
    c.canceled_reason,
    j.kind                     AS job_kind,
    CASE WHEN j.trigger_kind = 'schedule'::job_trigger_kind THEN j.trigger END
                               AS schedule_path,
    j.permissioned_as,
    COALESCE(c.flow_status, c.workflow_as_code_status) AS flow_status,
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
    c.result_columns,
    j.script_entrypoint_override,
    j.preprocessed
FROM v2_job_completed c
     JOIN v2_job j USING (id)
;
