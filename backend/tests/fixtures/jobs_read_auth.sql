-- Fixture for the single-job read authorization regression test
-- (see tests/jobs_read_auth.rs).
--
-- Users available from `base`:
--   test-user   (admin,     token SECRET_TOKEN)
--   test-user-2 (User,      token SECRET_TOKEN_2)  -- owner of the secret script
--   test-user-3 (User,      token SECRET_TOKEN_3)  -- the unprivileged "viewer"
--
-- test-user-3 is NOT a member of any folder/group granting access to
-- `u/test-user-2/...`, so under the same RLS as `jobs/list` they cannot see any
-- of these jobs unless they created them.

-- A tag-scoped token for test-user-2 (who can read both VICTIM (tag 'deno') and
-- the flow (tag 'flow')). The `if_jobs:filter_tags:deno` modifier restricts it to
-- the 'deno' tag, so it must NOT be able to mint a share token for the 'flow' job.
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES (
    encode(sha256('SCOPED_DENO_TOKEN'::bytea), 'hex'), 'SCOPED_DEN', 'SCOPED_DENO_TOKEN',
    'test2@windmill.dev', 'scoped deno token', false,
    ARRAY['jobs:read', 'if_jobs:filter_tags:deno']
);

-- App embed token for the admin viewer (test-user). Mirrors a minted sandboxed
-- low-code app token: carries the `app_embed` sentinel plus the embed scope set.
-- Used to assert the token is confined to jobs the viewer LAUNCHED, not every job
-- the (admin) viewer could otherwise read.
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES (
    encode(sha256('EMBED_APP_TOKEN'::bytea), 'hex'), 'EMBED_APP_', 'EMBED_APP_TOKEN',
    'test@windmill.dev', 'app embed token', false,
    ARRAY['apps:run', 'jobs:read', 'app_embed', 'resources:run', 'users:read', 'folders:read']
);

-- A completed app-component job LAUNCHED BY the admin viewer (created_by =
-- test-user), running as the app owner. The embed token must keep reading its own
-- launched job (the `created_by == viewer` fast path).
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner, args
) VALUES (
    '12121212-1212-1212-1212-121212121212', 'test-workspace', 'test-user',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/app_component', 'deno', false,
    '{"own": "arg"}'
);
INSERT INTO public.v2_job_completed (id, workspace_id, duration_ms, status, result) VALUES
    ('12121212-1212-1212-1212-121212121212', 'test-workspace', 1000, 'success'::job_status,
     '{"own": "EMBED_OWN_RESULT"}');

-- A QUEUED job launched by the admin embed viewer (created_by = test-user). The
-- embed token may cancel its own launched job; it must NOT cancel another user's.
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner
) VALUES (
    '13131313-1313-1313-1313-131313131313', 'test-workspace', 'test-user',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/app_component', 'deno', false
);
INSERT INTO public.v2_job_queue (id, workspace_id, scheduled_for, running, tag) VALUES
    ('13131313-1313-1313-1313-131313131313', 'test-workspace', '2023-01-01 00:00:00', false, 'deno');

-- RUNNING job: queued (no completed row) and owned by test-user-2. Used to check
-- that `completed/get_result_maybe?get_started=true` authorizes before disclosing
-- running-state to a non-reader.
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner
) VALUES (
    '77777777-7777-7777-7777-777777777777', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/running_secret', 'deno', true
);
INSERT INTO public.v2_job_queue (id, workspace_id, scheduled_for, running, tag) VALUES
    ('77777777-7777-7777-7777-777777777777', 'test-workspace', '2023-01-01 00:00:00', true, 'deno');

-- 1. VICTIM job: a completed run of test-user-2's private script, e.g. produced
--    by a public HTTP trigger. `created_by` is the route identity (test-user-2),
--    NOT the viewer; `permissioned_as`/`runnable_path` sit in test-user-2's
--    namespace; `visible_to_owner` is true. Its args + result carry secrets.
--    Pre-fix, test-user-3 could read all of these by UUID.
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner, args
) VALUES (
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/secret_script', 'deno', true,
    '{"secret": "LEAK_TEST_ARGS"}'
);
INSERT INTO public.v2_job_completed (
    id, workspace_id, duration_ms, status, result
) VALUES (
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'test-workspace', 1000,
    'success'::job_status, '{"secret": "RESULT_SECRET"}'
);
INSERT INTO public.job_logs (job_id, workspace_id, logs) VALUES
    ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'test-workspace', 'secret logs LEAK_TEST_LOGS');

-- 2. APP-style job: run by the viewer (test-user-3) on behalf of an app whose
--    policy executes as test-user-2. `created_by` is the launching viewer, but
--    `permissioned_as`/`runnable_path` are the app owner's and
--    `visible_to_owner` is false (apps hide their component runs from the runs
--    list). This is the case that must KEEP working after the fix: the viewer
--    polls their own component result by UUID.
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner, args
) VALUES (
    'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'test-workspace', 'test-user-3',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/app_component', 'deno', false,
    '{"app_arg": "ok"}'
);
INSERT INTO public.v2_job_completed (
    id, workspace_id, duration_ms, status, result
) VALUES (
    'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'test-workspace', 1000,
    'success'::job_status, '{"app_result": "visible_to_launcher"}'
);

-- 3. ANONYMOUS job: a public-trigger run whose creator is `anonymous`. Reading
--    it without authentication must keep working (unchanged behavior).
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner, args
) VALUES (
    'cccccccc-cccc-cccc-cccc-cccccccccccc', 'test-workspace', 'anonymous',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/public_trigger', 'deno', true,
    '{"public": "arg"}'
);
INSERT INTO public.v2_job_completed (
    id, workspace_id, duration_ms, status, result
) VALUES (
    'cccccccc-cccc-cccc-cccc-cccccccccccc', 'test-workspace', 1000,
    'success'::job_status, '{"public": "result"}'
);

-- 4. FLOW + STEP: test-user-3 has *read* access to folder `shared` (extra_perms),
--    so they can see flow `f/shared/flow1` (run by test-user-2) even though they
--    did not launch it. The flow's STEP job runs the inner script
--    `u/test-user-2/inner_secret` (test-user-3 has NO direct ACL on it) and is
--    not in their list. Visibility must be INHERITED from the flow root: being
--    able to see the flow means being able to inspect its steps (the flow-run UI
--    fetches each step by id). This guards against the fix over-blocking.
INSERT INTO public.folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'shared', 'Shared Folder', '{"u/test-user-2"}',
        '{"u/test-user-3": false}', 'test-user-2');

INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner
) VALUES (
    'dddddddd-dddd-dddd-dddd-dddddddddddd', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'flow', 'deno', 'f/shared/flow1', 'flow', true
);
INSERT INTO public.v2_job_completed (
    id, workspace_id, duration_ms, status, result
) VALUES (
    'dddddddd-dddd-dddd-dddd-dddddddddddd', 'test-workspace', 1000,
    'success'::job_status, '{"flow": "done"}'
);

INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner,
    parent_job, root_job, flow_innermost_root_job, args
) VALUES (
    'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/inner_secret', 'deno', true,
    'dddddddd-dddd-dddd-dddd-dddddddddddd', 'dddddddd-dddd-dddd-dddd-dddddddddddd',
    'dddddddd-dddd-dddd-dddd-dddddddddddd', '{"step_arg": "x"}'
);
INSERT INTO public.v2_job_completed (
    id, workspace_id, duration_ms, status, result
) VALUES (
    'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee', 'test-workspace', 1000,
    'success'::job_status, '{"step": "STEP_RESULT_INHERITED"}'
);

-- 5. DEEP NESTING / MIDDLE-LAYER VISIBILITY: top flow `f/secret/top` is NOT
--    visible to test-user-3; it has a sub-flow step `f/shared/mid` that IS visible
--    (folder `shared`); and that sub-flow has its own leaf step running
--    `u/test-user-2/deep_secret` (not visible). The leaf's `root_job` points at the
--    *outermost* top (not visible), so visibility must come from the *intermediate*
--    sub-flow the user can see — which requires walking the full parent chain, not
--    just [self, root].
INSERT INTO public.folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'secret', 'Secret Folder', '{"u/test-user-2"}', '{}', 'test-user-2');

-- top flow (not visible to test-user-3)
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner
) VALUES (
    'ffffffff-ffff-ffff-ffff-ffffffffffff', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'flow', 'deno', 'f/secret/top', 'flow', true
);
-- intermediate sub-flow (visible via folder `shared`), child of top
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner,
    parent_job, root_job, flow_innermost_root_job
) VALUES (
    '99999999-9999-9999-9999-999999999999', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'flow', 'deno', 'f/shared/mid', 'flow', true,
    'ffffffff-ffff-ffff-ffff-ffffffffffff', 'ffffffff-ffff-ffff-ffff-ffffffffffff',
    'ffffffff-ffff-ffff-ffff-ffffffffffff'
);
-- leaf step of the sub-flow; runnable not visible, root_job = outermost top (not visible)
INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, permissioned_as, permissioned_as_email,
    kind, script_lang, runnable_path, tag, visible_to_owner,
    parent_job, root_job, flow_innermost_root_job
) VALUES (
    '88888888-8888-8888-8888-888888888888', 'test-workspace', 'test-user-2',
    '2023-01-01 00:00:00', 'u/test-user-2', 'test2@windmill.dev',
    'script', 'deno', 'u/test-user-2/deep_secret', 'deno', true,
    '99999999-9999-9999-9999-999999999999', 'ffffffff-ffff-ffff-ffff-ffffffffffff',
    '99999999-9999-9999-9999-999999999999'
);
INSERT INTO public.v2_job_completed (id, workspace_id, duration_ms, status, result) VALUES
    ('ffffffff-ffff-ffff-ffff-ffffffffffff', 'test-workspace', 1000, 'success'::job_status,
     '{"top": "TOP_SECRET_RESULT"}'),
    ('99999999-9999-9999-9999-999999999999', 'test-workspace', 1000, 'success'::job_status,
     '{"mid": "MID_RESULT"}'),
    ('88888888-8888-8888-8888-888888888888', 'test-workspace', 1000, 'success'::job_status,
     '{"deep": "DEEP_STEP_INHERITED"}');
