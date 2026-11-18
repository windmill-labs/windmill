-- Fixture for the variable-value cache RLS regression test.
-- Extends base.sql (which defines test-user [admin], test-user-2, test-user-3
-- and their tokens).
--
-- A folder `secret` is readable ONLY by test-user-2 (via extra_perms). It holds a
-- variable that test-user-2 can read but test-user-3 cannot. A cache entry warmed
-- by test-user-2 with allow_cache=true must never be served back to test-user-3.

INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'secret', 'Secret Folder', '{}',
        '{"u/test-user-2": true}', 'test-user');

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'f/secret/cache_target_var', 'LEAKED_VAR_SECRET', false,
        'Folder-gated variable', '{}');
