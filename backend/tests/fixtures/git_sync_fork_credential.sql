-- A parent whose git-sync repository has a recorded credential, and the three
-- workspace shapes the qualification predicate has to tell apart.

INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('parent-ws', 'parent-ws', 'test-user', NULL),
    ('fork-ws',   'fork-ws',   'test-user', 'parent-ws'),
    ('errored-fork-ws', 'errored-fork-ws', 'test-user', 'parent-ws'),
    ('orphan-ws', 'orphan-ws', 'test-user', NULL);

-- A stored credential is encrypted with its own workspace's key.
INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('parent-ws', 'cloud', 'parent-key'),
    ('fork-ws', 'cloud', 'fork-key'),
    ('errored-fork-ws', 'cloud', 'errored-fork-key'),
    ('orphan-ws', 'cloud', 'orphan-key');

-- The parent holds the credential.
INSERT INTO workspace_settings (workspace_id, git_sync) VALUES
    ('parent-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo",
        "credential":{"provider":"gitlab","rotatable":true,"checked_at":1788500000}}]}'),

-- A fork inherits the repository but not the credential: this is what
-- clone_workspace_data leaves behind.
    ('fork-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo"}]}'),

-- A fork whose own credential has since failed. Its own standing must win over
-- the parent's healthy record rather than being papered over.
    ('errored-fork-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo",
        "credential":{"provider":"gitlab","rotatable":false,"checked_at":1788500000,
        "error":"GitLab no longer accepts this token"}}]}'),

-- No credential and no parent to borrow one from.
    ('orphan-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo"}]}');
