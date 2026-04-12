-- Fixture for fork review requests integration tests.
-- Two workspaces: 'parent-ws' (source) and 'fork-ws' (fork). The fork has a
-- parent_workspace_id pointing at parent-ws. wm_deployers exists in the
-- parent with one deployer member. Several usr rows exist: frev-admin,
-- frev-owner, frev-deployer, frev-random. Users get distinct emails to
-- avoid collisions with rows inserted by migrations.

INSERT INTO workspace (id, name, owner) VALUES
    ('parent-ws', 'Parent WS', 'frev-admin');
INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('fork-ws', 'Fork WS', 'frev-admin', 'parent-ws');

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
    VALUES ('frev-admin@windmill.dev', 'x', 'password', true, true, 'Frev Admin', 'frev-admin');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('frev-deployer@windmill.dev', 'x', 'password', false, true, 'Frev Deployer');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('frev-owner@windmill.dev', 'x', 'password', false, true, 'Frev Owner');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('frev-random@windmill.dev', 'x', 'password', false, true, 'Frev Random');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('parent-ws', 'frev-admin@windmill.dev', 'frev-admin', true, 'Admin'),
    ('parent-ws', 'frev-deployer@windmill.dev', 'frev-deployer', false, 'User'),
    ('parent-ws', 'frev-random@windmill.dev', 'frev-random', false, 'User'),
    ('fork-ws', 'frev-admin@windmill.dev', 'frev-admin', true, 'Admin'),
    ('fork-ws', 'frev-owner@windmill.dev', 'frev-owner', false, 'User'),
    ('fork-ws', 'frev-deployer@windmill.dev', 'frev-deployer', false, 'User'),
    ('fork-ws', 'frev-random@windmill.dev', 'frev-random', false, 'User');

INSERT INTO usr_to_group(workspace_id, group_, usr) VALUES
    ('parent-ws', 'wm_deployers', 'frev-deployer');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FREV_ADMIN_TOKEN'::bytea), 'hex'), 'FREV_ADMIN', 'FREV_ADMIN_TOKEN', 'frev-admin@windmill.dev', 't', true);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FREV_DEPLOYER_TOKEN'::bytea), 'hex'), 'FREV_DEPLO', 'FREV_DEPLOYER_TOKEN', 'frev-deployer@windmill.dev', 't', false);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FREV_OWNER_TOKEN'::bytea), 'hex'), 'FREV_OWNER', 'FREV_OWNER_TOKEN', 'frev-owner@windmill.dev', 't', false);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('FREV_RANDOM_TOKEN'::bytea), 'hex'), 'FREV_RANDO', 'FREV_RANDOM_TOKEN', 'frev-random@windmill.dev', 't', false);

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;

-- Seed a workspace_diff row so anchored comments can validate against an
-- existing (kind, path) tuple for this (parent, fork) pair.
INSERT INTO workspace_diff
    (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
VALUES ('parent-ws', 'fork-ws', 'f/shared/script1', 'script', 1, 0, true);
