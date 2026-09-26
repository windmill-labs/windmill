-- Add-on fixture for job_provenance.rs (combine with `base.sql`). A deployed script
-- f/t/tool run as a step under three parents: the deployed flow f/t/agent, a flow
-- preview claiming that path, and f/t/agent running the version of another flow.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
VALUES ('test-workspace', 1111111111, 'f/t/tool', 'echo tool', 'bash', 'script', 'test-user', '{}', '', '', '');

INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms, versions)
VALUES
    ('test-workspace', 'f/t/agent', '', '', '{"modules":[]}', 'test-user', NOW(), '{}', '{}', ARRAY[2222222222::bigint]),
    ('test-workspace', 'u/test-user/evil', '', '', '{"modules":[]}', 'test-user', NOW(), '{}', '{}', ARRAY[3333333333::bigint]);

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES
    (2222222222, 'test-workspace', 'f/t/agent', '{"modules":[]}', '{}', 'test-user', NOW()),
    (3333333333, 'test-workspace', 'u/test-user/evil', '{"modules":[]}', '{}', 'test-user', NOW());

INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-000000000001', 'test-workspace', 'flow', 'f/t/agent', 2222222222, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000002', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000003', 'test-workspace', 'flowpreview', 'f/t/agent', NULL, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000004', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000003', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000005', 'test-workspace', 'flow', 'f/t/agent', 3333333333, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000006', 'test-workspace', 'script', 'f/t/tool', 1111111111, '3bb0c0de-0000-4000-8000-000000000005', 'test-user', 'u/test-user', 'test@windmill.dev');
