-- Fixture for WM_END_USER_EMAIL tests
-- Sets up 3 users with different workspace memberships:
--   1. test@windmill.dev - in test-workspace (from base.sql)
--   2. other-ws@windmill.dev - in other-workspace only
--   3. no-ws@windmill.dev - not in any workspace

-- Second workspace for cross-workspace user
INSERT INTO workspace (id, name, owner)
VALUES ('other-workspace', 'other-workspace', 'other-ws-user');

INSERT INTO workspace_key(workspace_id, kind, key)
VALUES ('other-workspace', 'cloud', 'other-key');

INSERT INTO workspace_settings (workspace_id)
VALUES ('other-workspace');

INSERT INTO group_ (workspace_id, name, summary, extra_perms)
VALUES ('other-workspace', 'all', 'All users', '{}');

-- User in other-workspace only (not in test-workspace)
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
VALUES ('other-ws@windmill.dev', 'hash', 'password', false, true, 'Other WS User');

INSERT INTO usr(workspace_id, email, username, is_admin, role)
VALUES ('other-workspace', 'other-ws@windmill.dev', 'other-ws-user', true, 'Admin');

INSERT INTO token(token, email, label, super_admin)
VALUES ('OTHER_WS_TOKEN', 'other-ws@windmill.dev', 'other ws token', false);

-- User not in any workspace
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
VALUES ('no-ws@windmill.dev', 'hash', 'password', false, true, 'No WS User');

INSERT INTO token(token, email, label, super_admin)
VALUES ('NO_WS_TOKEN', 'no-ws@windmill.dev', 'no ws token', false);

-- Script that returns WM_END_USER_EMAIL (public via extra_perms)
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, kind, extra_perms)
VALUES (
    'test-workspace', 'test-user',
    'export function main() { return Deno.env.get("WM_END_USER_EMAIL") || ""; }',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    'Returns WM_END_USER_EMAIL', '', 'f/test/get_end_user_email', 900001, 'deno', '', 'script',
    '{"g/all": true}'
);

-- Flow that returns WM_END_USER_EMAIL (public via extra_perms)
INSERT INTO flow (workspace_id, summary, description, path, versions, schema, value, edited_by, extra_perms)
VALUES (
    'test-workspace', 'Returns WM_END_USER_EMAIL', '', 'f/test/get_end_user_email_flow', '{900002}',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"type": "rawscript", "language": "deno", "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }", "input_transforms": {}}}]}',
    'test-user',
    '{"g/all": true}'
);

INSERT INTO flow_version (id, workspace_id, path, schema, value, created_by)
VALUES (
    900002, 'test-workspace', 'f/test/get_end_user_email_flow',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"type": "rawscript", "language": "deno", "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }", "input_transforms": {}}}]}',
    'test-user'
);
