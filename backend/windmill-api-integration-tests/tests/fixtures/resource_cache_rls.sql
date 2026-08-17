-- Fixture for the resource-value interpolation cache RLS regression test.
-- Extends base.sql (which defines test-user [admin], test-user-2, test-user-3
-- and their tokens).
--
-- A folder `secret` is readable ONLY by test-user-2 (via extra_perms). It holds a
-- variable and a resource that interpolates it. test-user-3 has no access to the
-- folder, so a cache entry warmed by test-user-2 with allow_cache=true must never
-- be served back to test-user-3.

INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'secret', 'Secret Folder', '{}',
        '{"u/test-user-2": true}', 'test-user');

-- A (non-secret) variable gated to the `secret` folder; its value gets interpolated
-- into the resource value below and ends up in the cached, already-resolved blob.
INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'f/secret/db_password', 'LEAKED_FOLDER_SECRET', false,
        'Folder-gated secret', '{}');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'f/secret/cache_target',
        '{"host": "db.internal", "password": "$var:f/secret/db_password"}',
        'Folder-gated resource referencing a folder-gated variable', 'object', '{}', 'test-user');
