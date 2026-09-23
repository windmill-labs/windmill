-- A three-deep fork chain whose schedule rows were cloned down at fork time.
-- The middle fork has since deleted its copy, so the leaf's schedule shares
-- its cron only with the root — the shape a direct-parent check misses.

INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
    ('sfc-root', 'sfc-root', 'sfc-admin', NULL),
    ('sfc-mid',  'sfc-mid',  'sfc-admin', 'sfc-root'),
    ('sfc-leaf', 'sfc-leaf', 'sfc-admin', 'sfc-mid');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES
    ('sfc-root', 'cloud', 'sfc-root-key'),
    ('sfc-mid',  'cloud', 'sfc-mid-key'),
    ('sfc-leaf', 'cloud', 'sfc-leaf-key');

INSERT INTO workspace_settings (workspace_id) VALUES
    ('sfc-root'), ('sfc-mid'), ('sfc-leaf');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
    ('sfc-root', 'all', 'All users', '{}'),
    ('sfc-mid',  'all', 'All users', '{}'),
    ('sfc-leaf', 'all', 'All users', '{}');

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
    VALUES ('sfc-admin@windmill.dev', 'x', 'password', true, true, 'SFC Admin', 'sfc-admin');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
    ('sfc-root', 'sfc-admin@windmill.dev', 'sfc-admin', true, 'Admin'),
    ('sfc-mid',  'sfc-admin@windmill.dev', 'sfc-admin', true, 'Admin'),
    ('sfc-leaf', 'sfc-admin@windmill.dev', 'sfc-admin', true, 'Admin');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
    VALUES (encode(sha256('SFC_ADMIN_TOKEN'::bytea), 'hex'), 'SFC_ADMIN_', 'SFC_ADMIN_TOKEN', 'sfc-admin@windmill.dev', 't', true);

-- Enabling pushes the next run, which needs the scheduled script to exist.
INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES
    ('sfc-leaf', 'sfc-admin', 'export async function main() { return "ok" }', '{}', '', '', 'f/shared/job', 7788001, 'deno', '');

INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled, script_path, args, is_flow, email, timezone, extra_perms, permissioned_as)
VALUES
    ('sfc-root', 'f/shared/nightly', 'sfc-admin', NOW(), '0 0 0 * * *', true,  'f/shared/job', '{}', false, 'sfc-admin@windmill.dev', 'UTC', '{}', 'u/sfc-admin'),
    ('sfc-leaf', 'f/shared/nightly', 'sfc-admin', NOW(), '0 0 0 * * *', false, 'f/shared/job', '{}', false, 'sfc-admin@windmill.dev', 'UTC', '{}', 'u/sfc-admin'),
    -- A path only the leaf has: nothing above shares it.
    ('sfc-leaf', 'f/shared/own',     'sfc-admin', NOW(), '0 0 0 * * *', false, 'f/shared/job', '{}', false, 'sfc-admin@windmill.dev', 'UTC', '{}', 'u/sfc-admin');

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;
