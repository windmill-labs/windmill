-- Fixture for refusing a rename onto a path a draft already occupies.
--
-- A deployed script at `u/test-user/mvtaken_a` (hash 7010 = 0x1b62), and a
-- never-deployed draft of test-user's own at `u/test-user/mvtaken_b`, the path
-- the rename will target. Nothing deployed lives at the target, so only the
-- draft can refuse the move.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
                    schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 7010, 'u/test-user/mvtaken_a',
        'export function main() { return 1 }',
        'deno', 'script', 'test-user', '{}', 'A', '', '', '{}');

INSERT INTO draft (workspace_id, path, typ, value, email)
VALUES ('test-workspace', 'u/test-user/mvtaken_b', 'script',
        '{"path": "u/test-user/mvtaken_b", "summary": "B", "content": ""}',
        'test@windmill.dev');
