-- Add up migration script here
CREATE OR REPLACE VIEW v2_as_queue AS
SELECT
    j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    q.started_at,
    q.scheduled_for,
    q.running,
    j.runnable_id              AS script_hash,
    j.runnable_path            AS script_path,
    j.args,
    j.raw_code,
    q.canceled_by IS NOT NULL  AS canceled,
    q.canceled_by,
    q.canceled_reason,
    r.ping                     AS last_ping,
    j.kind                     AS job_kind,
    CASE WHEN j.trigger_kind = 'schedule'::job_trigger_kind THEN j.trigger END
                               AS schedule_path,
    j.permissioned_as,
    COALESCE(s.flow_status, s.workflow_as_code_status) AS flow_status,
    j.raw_flow,
    j.flow_step_id IS NOT NULL AS is_flow_step,
    j.script_lang              AS language,
    q.suspend,
    q.suspend_until,
    j.same_worker,
    j.raw_lock,
    j.pre_run_error,
    j.permissioned_as_email    AS email,
    j.visible_to_owner,
    r.memory_peak              AS mem_peak,
    j.flow_innermost_root_job  AS root_job,
    s.flow_leaf_jobs           AS leaf_jobs,
    j.tag,
    j.concurrent_limit,
    j.concurrency_time_window_s,
    j.timeout,
    j.flow_step_id,
    j.cache_ttl,
    j.priority,
    NULL::TEXT                 AS logs,
    j.script_entrypoint_override,
    j.preprocessed
FROM v2_job_queue q
     JOIN v2_job j USING (id)
     LEFT JOIN v2_job_runtime r USING (id)
     LEFT JOIN v2_job_status s USING (id)
;
