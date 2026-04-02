-- Test data for offboarding integration tests

-- Folder for reassign-to-folder tests
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'test-folder', 'Test Folder', ARRAY['u/test-user'], '{}', 'test-user');

-- Scripts owned by test-user-2
INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, schema, language, kind, lock, extra_perms)
VALUES
  ('test-workspace', 1001, 'u/test-user-2/script_a', 'Script A', '', 'print("a")', 'test-user-2', '{}', 'python3', 'script', '', '{}'),
  ('test-workspace', 1002, 'u/test-user-2/script_b', 'Script B', '', 'print("b")', 'test-user-2', '{}', 'python3', 'script', '', '{}');

-- Flow owned by test-user-2
INSERT INTO flow (workspace_id, path, summary, description, value, schema, edited_by, edited_at, extra_perms, versions)
VALUES ('test-workspace', 'u/test-user-2/flow_a', 'Flow A', '', '{"modules":[]}', '{}', 'test-user-2', NOW(), '{}', ARRAY[]::BIGINT[]);

-- Resource owned by test-user-2
INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user-2/res_a', '{"key": "val"}', 'Resource A', 'object', '{}', 'test-user-2');

-- Variable owned by test-user-2
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user-2/var_a', 'my_value', false, 'Variable A', '{}');

-- Schedule owned by test-user-2
INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, permissioned_as, extra_perms)
VALUES ('test-workspace', 'u/test-user-2/sched_a', 'test-user-2', NOW(), '0 * * * *', 'UTC', false, 'u/test-user-2/script_a', false, 'test2@windmill.dev', 'u/test-user-2', '{}');

-- Workspace-scoped token owned by test-user-2
INSERT INTO token (token_hash, token_prefix, token, email, label, workspace_id, owner)
VALUES (encode(sha256('OFFBOARD_TOKEN_1'::bytea), 'hex'), 'OFFBOARD_T', 'OFFBOARD_TOKEN_1', 'test2@windmill.dev', 'offboard test', 'test-workspace', 'u/test-user-2');

-- HTTP trigger under test-user-2's path (tests dynamic trigger queries)
INSERT INTO http_trigger (workspace_id, path, route_path, route_path_key, script_path, is_flow, edited_by, permissioned_as, edited_at, extra_perms, http_method, authentication_method, mode)
VALUES ('test-workspace', 'u/test-user-2/webhook_a', '/test-webhook', 'test-webhook', 'u/test-user-2/script_a', false, 'test-user-2', 'u/test-user-2', NOW(), '{}', 'post', 'none', 'enabled');

-- Script with extra_perms referencing test-user-2 (tests extra_perms cleanup)
INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, schema, language, kind, lock, extra_perms)
VALUES ('test-workspace', 1005, 'f/test-folder/shared_script', 'Shared Script', '', 'print("shared")', 'test-user', '{}', 'python3', 'script', '', '{"u/test-user-2": true}');

-- Schedule NOT under test-user-2 path but permissioned_as = u/test-user-2 (tests operator update)
INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, permissioned_as, extra_perms)
VALUES ('test-workspace', 'f/test-folder/sched_shared', 'test-user', NOW(), '0 * * * *', 'UTC', false, 'f/test-folder/shared_script', false, 'test@windmill.dev', 'u/test-user-2', '{}');

-- HTTP trigger NOT under test-user-2 path but permissioned_as = u/test-user-2 (tests line 232 + 951)
INSERT INTO http_trigger (workspace_id, path, route_path, route_path_key, script_path, is_flow, edited_by, permissioned_as, edited_at, extra_perms, http_method, authentication_method, mode)
VALUES ('test-workspace', 'f/test-folder/webhook_shared', '/shared-webhook', 'shared-webhook', 'f/test-folder/shared_script', false, 'test-user', 'u/test-user-2', NOW(), '{"u/test-user-2": true}', 'post', 'none', 'enabled');

-- For conflict test: script at the target path
INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, schema, language, kind, lock, extra_perms)
VALUES ('test-workspace', 1003, 'u/test-user/conflict_script', 'Conflict Script', '', 'print("conflict")', 'test-user', '{}', 'python3', 'script', '', '{}');

-- Script under test-user-2 that would conflict
INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, schema, language, kind, lock, extra_perms)
VALUES ('test-workspace', 1004, 'u/test-user-2/conflict_script', 'Conflict Script 2', '', 'print("conflict2")', 'test-user-2', '{}', 'python3', 'script', '', '{}');
