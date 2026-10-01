-- Add-on fixture for job_provenance.rs (combine with `base.sql` and `job_provenance.sql`):
-- past versions of f/t/tool and f/t/agent, and a deleted version of f/t/tool.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, created_at, archived)
VALUES ('test-workspace', 1111111110, 'f/t/tool', 'echo old tool', 'bash', 'script', 'test-user', '{}', '', '', '', NOW() - interval '1 day', true);

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, created_at, archived, deleted)
VALUES ('test-workspace', 1111111109, 'f/t/tool', '', 'bash', 'script', 'test-user', '{}', '', '', '', NOW() - interval '2 days', true, true);

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES (2222222221, 'test-workspace', 'f/t/agent', '{"modules":[]}', '{}', 'test-user', NOW() - interval '1 day');

UPDATE flow SET versions = ARRAY[2222222221::bigint, 2222222222::bigint]
WHERE workspace_id = 'test-workspace' AND path = 'f/t/agent';

-- Flow nodes of f/t/agent: the current version references 4444444442 (an inline step)
-- and 4444444444 (a loop body, holding the step 4444444446 and a nested loop whose body
-- 4444444447 holds the step 4444444448); 4444444441 and 4444444443 are from a past version.
INSERT INTO flow_node (id, workspace_id, hash, path, code, flow)
VALUES
    (4444444441, 'test-workspace', 1, 'f/t/agent', 'echo old step', NULL),
    (4444444442, 'test-workspace', 2, 'f/t/agent', 'echo step', NULL),
    (4444444443, 'test-workspace', 3, 'f/t/agent', NULL, '{"modules": []}'),
    (4444444444, 'test-workspace', 4, 'f/t/agent', NULL, '{"modules": [
        {"id": "b", "value": {"type": "flowscript", "id": 4444444446, "language": "bash"}},
        {"id": "c", "value": {"type": "forloopflow", "modules": [], "modules_node": 4444444447}}
    ]}'),
    (4444444446, 'test-workspace', 6, 'f/t/agent', 'echo body step', NULL),
    (4444444447, 'test-workspace', 7, 'f/t/agent', NULL, '{"modules": [
        {"id": "d", "value": {"type": "flowscript", "id": 4444444448, "language": "bash"}}
    ]}'),
    (4444444448, 'test-workspace', 8, 'f/t/agent', 'echo nested step', NULL);

INSERT INTO flow_version_lite (id, value)
VALUES (2222222222, '{"modules": [
    {"id": "a", "value": {"type": "flowscript", "id": 4444444442, "language": "bash"}},
    {"id": "l", "value": {"type": "forloopflow", "modules": [], "modules_node": 4444444444}}
]}');

INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    -- inline steps under the current flow version: the current node, and a past one
    -- kept by a nested restart
    ('3bb0c0de-0000-4000-8000-000000000105', 'test-workspace', 'flowscript', 'f/t/agent/a', 4444444442, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000106', 'test-workspace', 'flowscript', 'f/t/agent/a', 4444444441, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- loop bodies restarted on their own: the current node, and a past one
    ('3bb0c0de-0000-4000-8000-000000000107', 'test-workspace', 'flownode', 'f/t/agent/l', 4444444444, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000108', 'test-workspace', 'flownode', 'f/t/agent/l', 4444444443, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- a step of the loop body, a nested loop body, and a step of that nested body
    ('3bb0c0de-0000-4000-8000-000000000109', 'test-workspace', 'flowscript', 'f/t/agent/l/b', 4444444446, '3bb0c0de-0000-4000-8000-000000000107', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000110', 'test-workspace', 'flownode', 'f/t/agent/l/c', 4444444447, '3bb0c0de-0000-4000-8000-000000000107', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000111', 'test-workspace', 'flowscript', 'f/t/agent/l/c/d', 4444444448, '3bb0c0de-0000-4000-8000-000000000110', 'test-user', 'u/test-user', 'test@windmill.dev');

INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    -- the past tool version, run under the current flow version
    ('3bb0c0de-0000-4000-8000-000000000101', 'test-workspace', 'script', 'f/t/tool', 1111111110, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the current tool version, run under the past flow version
    ('3bb0c0de-0000-4000-8000-000000000102', 'test-workspace', 'flow', 'f/t/agent', 2222222221, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000103', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000102', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the deleted tool version
    ('3bb0c0de-0000-4000-8000-000000000104', 'test-workspace', 'script', 'f/t/tool', 1111111109, NULL, 'test-user', 'u/test-user', 'test@windmill.dev');
