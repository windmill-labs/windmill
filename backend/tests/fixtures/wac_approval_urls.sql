-- A Workflow-as-Code job in the queue, suspended on a wait_for_approval step
-- (see tests/wac_approval_urls.rs). WAC parents are plain script jobs with no
-- parent_job, which is what makes get_flow_info_for_resume treat them as WAC.
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner
) VALUES (
    'a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1', 'test-workspace', 'test-user',
    '2023-01-01 00:00:00', 'u/test-user', 'test@windmill.dev',
    'script', 'bun', 'u/test-user/wac_workflow', 'bun', true
);
INSERT INTO public.v2_job_queue (id, workspace_id, scheduled_for, running, suspend, tag) VALUES
    ('a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1', 'test-workspace', '2023-01-01 00:00:00', true, 1, 'bun');
