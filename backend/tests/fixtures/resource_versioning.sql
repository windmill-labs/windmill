-- Fixture for the resource version-history regression test.
-- Just a workspace + admin user + token; the test drives the real resource
-- endpoints and asserts what does and does not land in resource_version.

INSERT INTO workspace (id, name, owner) VALUES
    ('rver-ws', 'RVER WS', 'rver-admin');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('rver-ws', 'cloud', 'rver-key');

INSERT INTO workspace_settings (workspace_id) VALUES
    ('rver-ws');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('rver-ws', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('rver-admin@windmill.dev', 'x', 'password', true, true, 'RVER Admin', 'rver-admin');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('rver-ws', 'rver-admin@windmill.dev', 'rver-admin', true, 'Admin');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('RVER_ADMIN_TOKEN'::bytea), 'hex'), 'RVER_ADMIN', 'RVER_ADMIN_TOKEN', 'rver-admin@windmill.dev', 't', true);
