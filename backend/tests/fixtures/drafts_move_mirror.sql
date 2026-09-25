-- Fixture for the two-path-key move test.
--
-- Two draft-only scripts owned by test-user, both parked at a generated storage
-- key the way a session-created draft is. One carries the `draft_path` mirror the
-- session editor writes while the typed path differs from that key; the other
-- carries no mirror at all, which is what a plain draft looks like.

INSERT INTO draft (workspace_id, path, typ, value, email) VALUES
    ('test-workspace', 'u/test-user/draft_mirror', 'script',
     '{"path": "u/test-user/friendly", "draft_path": "u/test-user/friendly",
       "content": "x", "language": "bun", "summary": "S"}',
     'test@windmill.dev'),
    ('test-workspace', 'u/test-user/draft_plain', 'script',
     '{"path": "u/test-user/draft_plain",
       "content": "x", "language": "bun", "summary": "S"}',
     'test@windmill.dev');
