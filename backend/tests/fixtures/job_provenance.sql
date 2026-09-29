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

-- An app script run of f/t/app stamped by a deployed-app run, and one that is not
-- (an app editor preview).
INSERT INTO v2_job (id, workspace_id, kind, runnable_path, trigger_kind, trigger, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-000000000007', 'test-workspace', 'appscript', 'f/t/app/comp', 'app', 'f/t/app', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000008', 'test-workspace', 'appscript', 'f/t/app/comp', NULL, NULL, 'test-user', 'u/test-user', 'test@windmill.dev');

-- Inline steps of the deployed flow f/t/agent run as previews (no flow node, or
-- DISABLE_FLOW_SCRIPT): a direct step, a step in a loop body, and a step whose path
-- leaves the flow's namespace.
INSERT INTO v2_job (id, workspace_id, kind, runnable_path, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-000000000009', 'test-workspace', 'preview', 'f/t/agent/a', '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-00000000000a', 'test-workspace', 'flowpreview', 'f/t/agent/loop-0', '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-00000000000b', 'test-workspace', 'preview', 'f/t/agent/loop-0/a', '3bb0c0de-0000-4000-8000-00000000000a', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-00000000000c', 'test-workspace', 'preview', 'f/t/agentx', '3bb0c0de-0000-4000-8000-000000000001', 'test-user', 'u/test-user', 'test@windmill.dev');

-- Steps of the flow preview claiming f/t/agent: one nested in a branch, and one whose
-- path names another item.
INSERT INTO v2_job (id, workspace_id, kind, runnable_path, parent_job, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-00000000000d', 'test-workspace', 'flowpreview', 'f/t/agent/branchone-1', '3bb0c0de-0000-4000-8000-000000000003', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-00000000000e', 'test-workspace', 'preview', 'f/t/agent/branchone-1/a', '3bb0c0de-0000-4000-8000-00000000000d', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-00000000000f', 'test-workspace', 'preview', 'f/prod/deploy', '3bb0c0de-0000-4000-8000-000000000003', 'test-user', 'u/test-user', 'test@windmill.dev');

-- A deployed app without app_script entries runs its inline script as a stamped preview.
INSERT INTO v2_job (id, workspace_id, kind, runnable_path, trigger_kind, trigger, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-000000000010', 'test-workspace', 'preview', 'f/t/app/comp', 'app', 'f/t/app', 'test-user', 'u/test-user', 'test@windmill.dev');

-- A step of the deployed flow whose modules came in its args, and a flow preview at the
-- bare `f` whose step names another item.
INSERT INTO v2_job (id, workspace_id, kind, runnable_path, parent_job, args, created_by, permissioned_as, permissioned_as_email)
VALUES
    ('3bb0c0de-0000-4000-8000-000000000011', 'test-workspace', 'preview', 'f/t/agent/m', '3bb0c0de-0000-4000-8000-000000000001', '{"_MODULES": {}}', 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000012', 'test-workspace', 'flowpreview', 'f', NULL, NULL, 'test-user', 'u/test-user', 'test@windmill.dev'),
    ('3bb0c0de-0000-4000-8000-000000000013', 'test-workspace', 'preview', 'f/prod/deploy', '3bb0c0de-0000-4000-8000-000000000012', NULL, 'test-user', 'u/test-user', 'test@windmill.dev');
