-- A parent whose git-sync repository has a recorded credential, and the workspace
-- shapes the credential lookup and the qualification predicate have to tell apart.
--
-- The credential itself is shared down the fork chain; the recorded *status* is
-- not, because it describes one repository and a fork can repoint its copy of the
-- resource. Forks get a status by fork creation copying it, which no fixture here
-- simulates, so a fork without one is a workspace nothing has checked yet.

INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('parent-ws', 'parent-ws', 'test-user', NULL),
    ('fork-ws',   'fork-ws',   'test-user', 'parent-ws'),
    -- A fork of a fork: the shape a fork of a dev workspace takes, and the one a
    -- parent-only lookup misses.
    ('deep-fork-ws', 'deep-fork-ws', 'test-user', 'fork-ws'),
    ('errored-fork-ws', 'errored-fork-ws', 'test-user', 'parent-ws'),
    ('orphan-ws', 'orphan-ws', 'test-user', NULL);

-- A stored credential is encrypted with its own workspace's key.
INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('parent-ws', 'cloud', 'parent-key'),
    ('fork-ws', 'cloud', 'fork-key'),
    ('deep-fork-ws', 'cloud', 'deep-fork-key'),
    ('errored-fork-ws', 'cloud', 'errored-fork-key'),
    ('orphan-ws', 'cloud', 'orphan-key');

-- The parent holds the credential.
INSERT INTO workspace_settings (workspace_id, git_sync) VALUES
    ('parent-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo",
        "credential":{"provider":"gitlab","rotatable":true,"checked_at":1788500000}}]}'),

-- A fork inherits the repository but not the credential: this is what
-- clone_workspace_data leaves behind.
    ('fork-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo"}]}'),

-- Two levels down, so neither the credential nor its status is one hop away.
    ('deep-fork-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo"}]}'),

-- A fork whose own credential has since failed. Its own standing must win over
-- the parent's healthy record rather than being papered over.
    ('errored-fork-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo",
        "credential":{"provider":"gitlab","rotatable":false,"checked_at":1788500000,
        "error":"GitLab no longer accepts this token"}}]}'),

-- No credential and no parent to borrow one from.
    ('orphan-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo"}]}');

-- The resource each repository entry names, all pointing at the same repository.
-- The errored fork carries its token in the URL, the way a repository configured
-- by hand does: a plain remote, whatever the chain above it holds.
INSERT INTO resource (workspace_id, path, value, resource_type) VALUES
    ('parent-ws',       'u/admin/repo', '{"url":"https://gitlab.com/grp/proj.git"}', 'git_repository'),
    ('fork-ws',         'u/admin/repo', '{"url":"https://gitlab.com/grp/proj.git"}', 'git_repository'),
    ('deep-fork-ws',    'u/admin/repo', '{"url":"https://gitlab.com/grp/proj.git"}', 'git_repository'),
    ('errored-fork-ws', 'u/admin/repo', '{"url":"https://oauth2:glpat-inline@gitlab.com/grp/proj.git"}', 'git_repository'),
    ('orphan-ws',       'u/admin/repo', '{"url":"https://gitlab.com/grp/proj.git"}', 'git_repository');
