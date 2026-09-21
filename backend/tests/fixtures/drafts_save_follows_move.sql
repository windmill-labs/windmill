-- Fixture for saving a draft to the path its item moved away from.
--
-- A deployed script at `u/test-user/follow_a` (hash 7030 = 0x1b76) with the
-- deployer's own draft on it. The test renames the script and then saves the
-- draft from an editor still bound to the old path.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
                    schema, summary, description, lock, extra_perms)
VALUES ('test-workspace', 7030, 'u/test-user/follow_a',
        'export function main() { return 1 }',
        'deno', 'script', 'test-user', '{}', 'A', '', '', '{}');

INSERT INTO draft (workspace_id, path, typ, value, email, base)
VALUES ('test-workspace', 'u/test-user/follow_a', 'script',
        '{"path": "u/test-user/follow_a", "parent_hash": "0000000000001b76", "summary": "A", "content": "draft"}',
        'test@windmill.dev', '0000000000001b76');

-- A draft-only script parked at a generated storage key, its typed path elsewhere.
INSERT INTO draft (workspace_id, path, typ, value, email)
VALUES ('test-workspace', 'u/test-user/draft_store', 'script',
        '{"path": "u/test-user/friendly", "draft_path": "u/test-user/friendly", "summary": "D", "content": "draft"}',
        'test@windmill.dev');

-- A teammate's draft on the same deployed script, forked from the same head. The
-- rename must carry it too, without touching the version it forked from.
INSERT INTO draft (workspace_id, path, typ, value, email, base)
VALUES ('test-workspace', 'u/test-user/follow_a', 'script',
        '{"path": "u/test-user/follow_a", "draft_path": "u/test-user/follow_a", "parent_hash": "0000000000001b76", "summary": "A", "content": "teammate draft"}',
        'test2@windmill.dev', '0000000000001b76');
