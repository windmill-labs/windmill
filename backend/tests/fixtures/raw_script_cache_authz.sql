-- Fixture for the raw-script content-cache authorization regression test.
-- Layered on top of `base` (test-workspace, admin `test-user`/SECRET_TOKEN, plain
-- member `test-user-2`/SECRET_TOKEN_2).
--
-- Folder `secret` is owned by `u/test-user` only and grants nothing through
-- `extra_perms`, so `f/secret/lib` is invisible to test-user-2 under the script
-- table's folder RLS policy.

INSERT INTO public.folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'secret', 'Secret Folder', '{"u/test-user"}', '{}', 'test-user');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, extra_perms) VALUES (
'test-workspace',
'test-user',
'TOP_SECRET_CONTENT = "leak-canary"
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/secret/lib', 424200, 'python3', '', '{}');

-- Nested one level deeper so a probe of the intermediate `f/secret/pkg` has the
-- >2 path segments that arm the folder-existence cache.
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, extra_perms) VALUES (
'test-workspace',
'test-user',
'LEAF = 1
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/secret/pkg/leaf', 424201, 'python3', '', '{}');
