-- Fixture for the RestrictDeployToDeployers protection rule test.
-- Uses a dedicated workspace id + dedicated token prefixes so the process-
-- global AUTH_CACHE and PROTECTION_RULES_CACHE lazy_statics don't race with
-- the parallel test_protection_rules test (which also uses test-workspace).

INSERT INTO workspace (id, name, owner) VALUES
    ('rdd-ws', 'Restrict Deploy WS', 'rdd-admin');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('rdd-ws', 'cloud', 'rdd-key');

INSERT INTO workspace_settings (workspace_id) VALUES
    ('rdd-ws');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('rdd-ws', 'all', 'All users', '{}'),
    ('rdd-ws', 'wm_deployers', 'Deployers', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('rdd-admin@windmill.dev', 'x', 'password', true, true, 'RDD Admin', 'rdd-admin');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('rdd-deployer@windmill.dev', 'x', 'password', false, true, 'RDD Deployer');
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('rdd-user@windmill.dev', 'x', 'password', false, true, 'RDD User');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('rdd-ws', 'rdd-admin@windmill.dev', 'rdd-admin', true, 'Admin'),
    ('rdd-ws', 'rdd-deployer@windmill.dev', 'rdd-deployer', false, 'User'),
    ('rdd-ws', 'rdd-user@windmill.dev', 'rdd-user', false, 'User');

INSERT INTO usr_to_group(workspace_id, group_, usr) VALUES
    ('rdd-ws', 'wm_deployers', 'rdd-deployer');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('RDD_ADMIN_TOKEN'::bytea), 'hex'), 'RDD_ADMIN_', 'RDD_ADMIN_TOKEN', 'rdd-admin@windmill.dev', 't', true);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('RDD_DEPLOYER_TOKEN'::bytea), 'hex'), 'RDD_DEPLOY', 'RDD_DEPLOYER_TOKEN', 'rdd-deployer@windmill.dev', 't', false);
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('RDD_USER_TOKEN'::bytea), 'hex'), 'RDD_USER_T', 'RDD_USER_TOKEN', 'rdd-user@windmill.dev', 't', false);

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;
