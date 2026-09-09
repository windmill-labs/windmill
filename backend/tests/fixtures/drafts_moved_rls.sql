-- Fixture for the "moved" draft answer under RLS.
--
-- Models a script that has already been moved out of a folder the saver can
-- reach (`mvrls_visible`, where test-user-2 is a writer) and into one they have
-- no permission on (`mvrls_secret`). A script move goes through `create_script`,
-- which archives the row in place and inserts a successor carrying the old hash
-- in `parent_hashes` — the state reproduced here.

INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'mvrls_visible', 'Visible', '{"u/test-user"}',
        '{"u/test-user": true, "u/test-user-2": true}', 'test-user');

-- No entry for test-user-2: the destination is invisible to them.
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
VALUES ('test-workspace', 'mvrls_secret', 'Secret', '{"u/test-user"}',
        '{"u/test-user": true}', 'test-user');

-- The pre-move row, archived in place at the old path. Hash 7001 = 0x1b59.
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
                    schema, summary, description, lock, extra_perms, archived)
VALUES ('test-workspace', 7001, 'f/mvrls_visible/s1',
        'export function main() { return 1 }',
        'deno', 'script', 'test-user', '{}', 'S1', '', '', '{}', true);

-- The post-move row, at the destination, pointing back at 7001.
INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
                    schema, summary, description, lock, extra_perms, parent_hashes)
VALUES ('test-workspace', 7002, 'f/mvrls_secret/s1',
        'export function main() { return 1 }',
        'deno', 'script', 'test-user', '{}', 'S1', '', '', '{}', '{7001}');
