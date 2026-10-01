-- Add-on fixture for job_provenance.rs (combine with `base.sql` and `job_provenance.sql`):
-- past versions of f/t/tool and f/t/agent, a deleted version of f/t/tool, and flow steps
-- of f/t/agent under a regular run and under restarts.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, created_at, archived)
VALUES ('test-workspace', 1111111110, 'f/t/tool', 'echo old tool', 'bash', 'script', 'test-user', '{}', '', '', '', NOW() - interval '1 day', true);

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, created_at, archived, deleted)
VALUES ('test-workspace', 1111111109, 'f/t/tool', '', 'bash', 'script', 'test-user', '{}', '', '', '', NOW() - interval '2 days', true, true);

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES (2222222221, 'test-workspace', 'f/t/agent', '{"modules":[]}', '{}', 'test-user', NOW() - interval '1 day');

UPDATE flow SET versions = ARRAY[2222222221::bigint, 2222222222::bigint]
WHERE workspace_id = 'test-workspace' AND path = 'f/t/agent';

INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    -- the past tool version, run under the current flow version
    ('3bb0c0de-0000-4000-8000-000000000101', 'test-workspace', 'script', 'f/t/tool', 1111111110, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the current tool version, run under the past flow version
    ('3bb0c0de-0000-4000-8000-000000000102', 'test-workspace', 'flow', 'f/t/agent', 2222222221, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000103', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000102', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the deleted tool version
    ('3bb0c0de-0000-4000-8000-000000000104', 'test-workspace', 'script', 'f/t/tool', 1111111109, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- under the current flow version: an inline step, a loop body and a step of that body
    ('3bb0c0de-0000-4000-8000-000000000105', 'test-workspace', 'flowscript', 'f/t/agent/a', 4444444442, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000106', 'test-workspace', 'flownode', 'f/t/agent/loop-0', 4444444444, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000107', 'test-workspace', 'flowscript', 'f/t/agent/loop-0/b', 4444444446, '3bb0c0de-0000-4000-8000-000000000106', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- an inline step under the past flow version
    ('3bb0c0de-0000-4000-8000-000000000108', 'test-workspace', 'flowscript', 'f/t/agent/a', 4444444441, '3bb0c0de-0000-4000-8000-000000000102', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- a loop body restarted on its own, and a step of it
    ('3bb0c0de-0000-4000-8000-000000000109', 'test-workspace', 'flownode', 'f/t/agent/loop-0', 4444444443, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000110', 'test-workspace', 'flowscript', 'f/t/agent/loop-0/b', 4444444449, '3bb0c0de-0000-4000-8000-000000000109', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- a body kept by a nested restart under the current flow version, and a step of it
    ('3bb0c0de-0000-4000-8000-000000000111', 'test-workspace', 'flownode', 'f/t/agent/branchone-1', 4444444445, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000112', 'test-workspace', 'flowscript', 'f/t/agent/branchone-1/c', 4444444447, '3bb0c0de-0000-4000-8000-000000000111', 'test-user', 'u/test-user', 'test@windmill.dev');

-- The restarted bodies: one still running, one completed.
INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running)
VALUES ('3bb0c0de-0000-4000-8000-000000000111', 'test-workspace', NOW(), true);

INSERT INTO v2_job_status (id, flow_status)
VALUES ('3bb0c0de-0000-4000-8000-000000000111',
    '{"restarted_from": {"flow_job_id": "3bb0c0de-0000-4000-8000-0000000000ff", "step_id": "br"}}');

INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, flow_status)
VALUES ('3bb0c0de-0000-4000-8000-000000000109', 'test-workspace', 0, 'success',
    '{"restarted_from": {"flow_job_id": "3bb0c0de-0000-4000-8000-0000000000fe", "step_id": "l"}}');
