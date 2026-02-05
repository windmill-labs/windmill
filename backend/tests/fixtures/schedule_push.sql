-- Fixture for schedule push tests
-- Sets up scripts, flows, users, and schedules needed to test push_scheduled_job

-- Password entries for auth resolution
INSERT INTO password (email, password_hash, login_type, super_admin, verified, name)
VALUES
    ('test@windmill.dev', 'dummy_hash', 'password', false, true, 'Test User'),
    ('obo@windmill.dev', 'dummy_hash', 'password', false, true, 'OBO User')
ON CONFLICT (email) DO NOTHING;

-- OBO user in workspace
INSERT INTO usr (workspace_id, email, username, is_admin, role)
VALUES ('test-workspace', 'obo@windmill.dev', 'obo-user', false, 'Developer')
ON CONFLICT (workspace_id, username) DO NOTHING;

-- A simple script
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, kind)
VALUES (
    'test-workspace', 'test-user',
    'export async function main() { return "ok"; }',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    'Test script', '', 'f/system/test_script', 100001, 'deno', '', 'script'
);

-- A script with on_behalf_of_email
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, kind, on_behalf_of_email)
VALUES (
    'test-workspace', 'test-user',
    'export async function main() { return "obo"; }',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    'OBO script', '', 'f/system/obo_script', 100002, 'deno', '', 'script', 'obo@windmill.dev'
);

-- A script with a tag
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, kind, tag)
VALUES (
    'test-workspace', 'test-user',
    'export async function main() { return "tagged"; }',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    'Tagged script', '', 'f/system/tagged_script', 100003, 'deno', '', 'script', 'custom-tag'
);

-- A script with timeout
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, kind, timeout)
VALUES (
    'test-workspace', 'test-user',
    'export async function main() { return "timeout"; }',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    'Timeout script', '', 'f/system/timeout_script', 100004, 'deno', '', 'script', 300
);

-- A flow
INSERT INTO flow (workspace_id, summary, description, path, versions, schema, value, edited_by)
VALUES (
    'test-workspace', 'Test flow', '', 'f/system/test_flow', '{200001}',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"path": "f/system/test_script", "type": "script", "input_transforms": {}}}]}',
    'test-user'
);

INSERT INTO flow_version (id, workspace_id, path, schema, value, created_by)
VALUES (
    200001, 'test-workspace', 'f/system/test_flow',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"path": "f/system/test_script", "type": "script", "input_transforms": {}}}]}',
    'test-user'
);

-- A flow with on_behalf_of_email
INSERT INTO flow (workspace_id, summary, description, path, versions, schema, value, edited_by, on_behalf_of_email)
VALUES (
    'test-workspace', 'OBO flow', '', 'f/system/obo_flow', '{200002}',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"path": "f/system/test_script", "type": "script", "input_transforms": {}}}]}',
    'test-user', 'obo@windmill.dev'
);

INSERT INTO flow_version (id, workspace_id, path, schema, value, created_by)
VALUES (
    200002, 'test-workspace', 'f/system/obo_flow',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
    '{"modules": [{"id": "a", "value": {"path": "f/system/test_script", "type": "script", "input_transforms": {}}}]}',
    'test-user'
);
