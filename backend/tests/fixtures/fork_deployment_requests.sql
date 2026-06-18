-- Fixture for fork deployment request integration tests.
-- Two workspaces: 'parent-ws' (source) and 'fork-ws' (fork, with
-- parent_workspace_id=parent-ws). wm_deployers exists in the parent with
-- one member. Users get distinct emails/usernames/tokens to avoid
-- collisions with rows inserted by migrations.

INSERT INTO workspace (id, name, owner) VALUES
    ('parent-ws', 'Parent WS', 'fdr-admin');
INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('fork-ws', 'Fork WS', 'fdr-admin', 'parent-ws');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('parent-ws', 'cloud', 'parent-key'),
    ('fork-ws', 'cloud', 'fork-key');

INSERT INTO workspace_settings (workspace_id) VALUES
    ('parent-ws'),
    ('fork-ws');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('parent-ws', 'all', 'All users', '{}'),
    ('parent-ws', 'wm_deployers', 'Deployers', '{}'),
    ('fork-ws', 'all', 'All users', '{}'),
    ('fork-ws', 'wm_deployers', 'Deployers', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('fdr-admin@windmill.dev', 'x', 'password', true, true, 'FDR Admin', 'fdr-admin');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('fdr-deployer@windmill.dev', 'x', 'password', false, true, 'FDR Deployer');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('fdr-owner@windmill.dev', 'x', 'password', false, true, 'FDR Owner');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('fdr-random@windmill.dev', 'x', 'password', false, true, 'FDR Random');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('parent-ws', 'fdr-admin@windmill.dev', 'fdr-admin', true, 'Admin'),
    ('parent-ws', 'fdr-deployer@windmill.dev', 'fdr-deployer', false, 'User'),
    ('parent-ws', 'fdr-random@windmill.dev', 'fdr-random', false, 'User'),
    ('fork-ws', 'fdr-admin@windmill.dev', 'fdr-admin', true, 'Admin'),
    ('fork-ws', 'fdr-owner@windmill.dev', 'fdr-owner', false, 'User'),
    ('fork-ws', 'fdr-deployer@windmill.dev', 'fdr-deployer', false, 'User'),
    ('fork-ws', 'fdr-random@windmill.dev', 'fdr-random', false, 'User');

INSERT INTO usr_to_group(workspace_id, group_, usr) VALUES
    ('parent-ws', 'wm_deployers', 'fdr-deployer');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FDR_ADMIN_TOKEN'::bytea), 'hex'), 'FDR_ADMIN_', 'FDR_ADMIN_TOKEN', 'fdr-admin@windmill.dev', 't', true);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FDR_DEPLOYER_TOKEN'::bytea), 'hex'), 'FDR_DEPLOY', 'FDR_DEPLOYER_TOKEN', 'fdr-deployer@windmill.dev', 't', false);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FDR_OWNER_TOKEN'::bytea), 'hex'), 'FDR_OWNER_', 'FDR_OWNER_TOKEN', 'fdr-owner@windmill.dev', 't', false);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FDR_RANDOM_TOKEN'::bytea), 'hex'), 'FDR_RANDOM', 'FDR_RANDOM_TOKEN', 'fdr-random@windmill.dev', 't', false);

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;

-- Seed a workspace_diff row so anchored comments can validate against an
-- existing (kind, path) tuple for this (parent, fork) pair.
INSERT INTO workspace_diff
    (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
VALUES ('parent-ws', 'fork-ws', 'f/shared/script1', 'script', 1, 0, true);
