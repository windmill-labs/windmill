-- A parent workspace whose auto-pulled repository was last saved by alice, and a fork of
-- it holding only carol, the non-admin who created it. aaron is an admin who sorts before
-- alice; bob is no longer an admin; dora is a workspace admin deactivated on the instance.
-- sam and sue are instance superadmins who are not members: sam's instance username is
-- carol's, sue's is unclaimed.

INSERT INTO workspace (id, name, owner) VALUES ('ap-parent', 'ap-parent', 'alice@windmill.dev');
INSERT INTO workspace (id, name, owner, parent_workspace_id)
    VALUES ('wm-fork-feat', 'feat', 'carol@windmill.dev', 'ap-parent');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('ap-parent', 'all', 'All users', '{}'),
    ('wm-fork-feat', 'all', 'All users', '{}');

INSERT INTO password (email, password_hash, login_type, super_admin, verified, name, disabled) VALUES
    ('aaron@windmill.dev', 'not-a-real-hash', 'password', false, true, 'aaron', false),
    ('alice@windmill.dev', 'not-a-real-hash', 'password', false, true, 'alice', false),
    ('bob@windmill.dev', 'not-a-real-hash', 'password', false, true, 'bob', false),
    ('carol@windmill.dev', 'not-a-real-hash', 'password', false, true, 'carol', false),
    ('dora@windmill.dev', 'not-a-real-hash', 'password', false, true, 'dora', true);

INSERT INTO password (email, password_hash, login_type, super_admin, verified, name, disabled, username) VALUES
    ('sam@windmill.dev', 'not-a-real-hash', 'password', true, true, 'sam', false, 'carol'),
    ('sue@windmill.dev', 'not-a-real-hash', 'password', true, true, 'sue', false, 'sue');

INSERT INTO usr (workspace_id, email, username, is_admin) VALUES
    ('ap-parent', 'aaron@windmill.dev', 'aaron', true),
    ('ap-parent', 'alice@windmill.dev', 'alice', true),
    ('ap-parent', 'bob@windmill.dev', 'bob', false),
    ('ap-parent', 'carol@windmill.dev', 'carol', false),
    ('ap-parent', 'dora@windmill.dev', 'dora', true),
    ('wm-fork-feat', 'carol@windmill.dev', 'carol', false);

INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by) VALUES
    ('ap-parent', 'u/alice/repo', '{"url": "https://github.com/test/repo.git", "branch": "main"}',
        'git_repository', '{}', 'alice'),
    ('wm-fork-feat', 'u/alice/repo', '{"url": "https://github.com/test/repo.git", "branch": "main"}',
        'git_repository', '{}', 'alice');

INSERT INTO workspace_settings (workspace_id, git_sync) VALUES
    ('ap-parent', '{"repositories":[{"git_repo_resource_path":"$res:u/alice/repo",
        "use_individual_branch":false,"group_by_folder":false,
        "auto_pull":{"enabled":true,"mode":"polling","sync_forks":true,
            "enabled_by":"alice@windmill.dev"}}]}'),
    ('wm-fork-feat', '{"repositories":[{"git_repo_resource_path":"$res:u/alice/repo",
        "use_individual_branch":false,"group_by_folder":false}]}');
