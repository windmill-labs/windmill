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

INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    -- the past tool version, run under the current flow version
    ('3bb0c0de-0000-4000-8000-000000000101', 'test-workspace', 'script', 'f/t/tool', 1111111110, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the current tool version, run under the past flow version
    ('3bb0c0de-0000-4000-8000-000000000102', 'test-workspace', 'flow', 'f/t/agent', 2222222221, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000103', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000102', 'test-user', 'u/test-user', 'test@windmill.dev'),
    -- the deleted tool version
    ('3bb0c0de-0000-4000-8000-000000000104', 'test-workspace', 'script', 'f/t/tool', 1111111109, NULL, 'test-user', 'u/test-user', 'test@windmill.dev');
