-- Fixture for the data table migration bookkeeping-grants regression test.
-- Workspace + admin token + a data table pointing at a postgres resource; the
-- test fills that resource in with credentials for a deliberately unprivileged
-- role, since the database name is allocated per test run.

INSERT INTO workspace (id, name, owner) VALUES
    ('dtmig-ws', 'DTMIG WS', 'dtmig-admin');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('dtmig-ws', 'cloud', 'dtmig-key');

INSERT INTO workspace_settings (workspace_id, datatable) VALUES
    ('dtmig-ws', '{"datatables": {"main": {"database": {"resource_type": "postgresql", "resource_path": "u/dtmig-admin/pg"}, "migrations_enabled": true}, "noschema": {"database": {"resource_type": "postgresql", "resource_path": "u/dtmig-admin/pg_noschema"}, "migrations_enabled": true}}}');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('dtmig-ws', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('dtmig-admin@windmill.dev', 'x', 'password', true, true, 'DTMIG Admin', 'dtmig-admin');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('dtmig-ws', 'dtmig-admin@windmill.dev', 'dtmig-admin', true, 'Admin');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('DTMIG_ADMIN_TOKEN'::bytea), 'hex'), 'DTMIG_ADM', 'DTMIG_ADMIN_TOKEN', 'dtmig-admin@windmill.dev', 't', true);

-- Non-admin member, to pin that the privilege report stays admin-only.
INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('dtmig-user@windmill.dev', 'x', 'password', false, true, 'DTMIG User', 'dtmig-user');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('dtmig-ws', 'dtmig-user@windmill.dev', 'dtmig-user', false, 'User');

INSERT INTO token(token_hash, token_prefix, token, email, label)
    VALUES (encode(sha256('DTMIG_USER_TOKEN'::bytea), 'hex'), 'DTMIG_USR', 'DTMIG_USER_TOKEN', 'dtmig-user@windmill.dev', 't');
