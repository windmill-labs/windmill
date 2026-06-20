-- Fixture for the draft NUL-byte write-sanitization regression test.
-- Just a workspace + admin user + token; the test itself POSTs a draft whose
-- value carries a U+0000 and asserts it is stored (and listed) NUL-free.

INSERT INTO workspace (id, name, owner) VALUES
    ('dnul-ws', 'DNUL WS', 'dnul-admin');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('dnul-ws', 'cloud', 'dnul-key');

INSERT INTO workspace_settings (workspace_id) VALUES
    ('dnul-ws');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('dnul-ws', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('dnul-admin@windmill.dev', 'x', 'password', true, true, 'DNUL Admin', 'dnul-admin');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('dnul-ws', 'dnul-admin@windmill.dev', 'dnul-admin', true, 'Admin');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('DNUL_ADMIN_TOKEN'::bytea), 'hex'), 'DNUL_ADMIN', 'DNUL_ADMIN_TOKEN', 'dnul-admin@windmill.dev', 't', true);
