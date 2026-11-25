-- A parent and its fork with distinct encryption keys. `wd-admin` is an admin of both, and
-- `wd-dev` is a member of both under different usernames. `wd-outsider` belongs to the parent
-- only.

INSERT INTO workspace (id, name, owner) VALUES ('wd-parent', 'Parent', 'wd-admin');
INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('wd-fork', 'Fork', 'wd-admin', 'wd-parent');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('wd-parent', 'cloud', 'parent-key-0000000000000000000000'),
    ('wd-fork', 'cloud', 'fork-key-00000000000000000000000000');

INSERT INTO workspace_settings (workspace_id) VALUES ('wd-parent'), ('wd-fork');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('wd-parent', 'all', 'All users', '{}'),
    ('wd-fork', 'all', 'All users', '{}'),
    ('wd-fork', 'forkonly', 'Only in the fork', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name) VALUES
    ('wd-admin@windmill.dev', 'x', 'password', false, true, 'Admin'),
    ('wd-dev@windmill.dev', 'x', 'password', false, true, 'Dev'),
    ('wd-outsider@windmill.dev', 'x', 'password', false, true, 'Outsider');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('wd-parent', 'wd-admin@windmill.dev', 'wd-admin', true, 'Admin'),
    ('wd-fork', 'wd-admin@windmill.dev', 'wd-admin', true, 'Admin'),
    ('wd-parent', 'wd-dev@windmill.dev', 'dev-in-parent', false, 'User'),
    ('wd-fork', 'wd-dev@windmill.dev', 'dev-in-fork', false, 'User'),
    ('wd-parent', 'wd-outsider@windmill.dev', 'wd-outsider', false, 'User');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
    (encode(sha256('WD_ADMIN_TOKEN'::bytea), 'hex'), 'WD_ADMIN_T', 'WD_ADMIN_TOKEN', 'wd-admin@windmill.dev', 't', false),
    (encode(sha256('WD_OUTSIDER_TOKEN'::bytea), 'hex'), 'WD_OUTSIDE', 'WD_OUTSIDER_TOKEN', 'wd-outsider@windmill.dev', 't', false);

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;
