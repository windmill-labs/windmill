-- Fixture for the flow step/sub-flow reference authorization test (WIN-2412).
--
-- A private folder only the admin can read holds a script and a flow. A plain
-- workspace member (test-user-3) has write access to `f/shared`, so they can
-- deploy and preview flows there, but must not be able to point a step at
-- anything inside `f/private`.

INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'private', 'Private Folder', '{"u/test-user"}', '{}', 'test-user');

INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'shared', 'Shared Folder', '{"u/test-user"}', '{"u/test-user-3": true}', 'test-user');

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 2412001, 'f/private/hidden',
        'export function main() { return "hidden"; }',
        'deno', 'script', 'test-user', '{}', 'Hidden script', '', '', '{}');

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 2412002, 'f/shared/visible',
        'export function main() { return "visible"; }',
        'deno', 'script', 'test-user', '{}', 'Visible script', '', '', '{}');

INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms, versions)
VALUES ('test-workspace', 'f/private/hiddenflow', 'Hidden flow', '',
        '{"modules": []}', 'test-user', NOW(), '{}', '{}', ARRAY[2412003::bigint]);

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES (2412003, 'test-workspace', 'f/private/hiddenflow', '{"modules": []}', '{}', 'test-user', NOW());
