-- Fixture for instance group auto-add tests
-- Sets up test data for testing the interaction between instance groups and workspace auto-add

-- Create additional workspaces for testing
INSERT INTO workspace (id, name, owner)
VALUES
    ('ws-with-auto-add', 'Workspace with Auto Add', 'admin'),
    ('ws-no-auto-add', 'Workspace without Auto Add', 'admin'),
    ('ws-multi-group', 'Workspace with Multiple Groups', 'admin')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_settings (workspace_id)
VALUES
    ('ws-with-auto-add'),
    ('ws-no-auto-add'),
    ('ws-multi-group')
ON CONFLICT DO NOTHING;

INSERT INTO workspace_key(workspace_id, kind, key)
VALUES
    ('ws-with-auto-add', 'cloud', 'key-auto-add'),
    ('ws-no-auto-add', 'cloud', 'key-no-auto'),
    ('ws-multi-group', 'cloud', 'key-multi')
ON CONFLICT DO NOTHING;

-- Create instance groups
INSERT INTO instance_group (name, summary, id)
VALUES
    ('engineering', 'Engineering team', 'eng-uuid-001'),
    ('sales', 'Sales team', 'sales-uuid-002'),
    ('admins', 'Admin group', 'admin-uuid-003')
ON CONFLICT DO NOTHING;

-- Create users in the password table (required for auto-add eligibility)
INSERT INTO password (email, password_hash, login_type, super_admin, verified, name)
VALUES
    ('alice@example.com', 'hash1', 'password', false, true, 'Alice'),
    ('bob@example.com', 'hash2', 'password', false, true, 'Bob'),
    ('charlie@example.com', 'hash3', 'password', false, true, 'Charlie'),
    ('dave@example.com', 'hash4', 'password', false, true, 'Dave'),
    ('admin@windmill.dev', 'hash5', 'password', true, true, 'Admin')
ON CONFLICT DO NOTHING;

-- Add users to instance groups
INSERT INTO email_to_igroup (email, igroup)
VALUES
    ('alice@example.com', 'engineering'),
    ('bob@example.com', 'engineering'),
    ('bob@example.com', 'sales'),
    ('charlie@example.com', 'sales'),
    ('dave@example.com', 'admins')
ON CONFLICT DO NOTHING;

-- Create admin user in workspaces for running tests
INSERT INTO usr (workspace_id, username, email, is_admin, operator)
VALUES
    ('ws-with-auto-add', 'admin', 'admin@windmill.dev', true, false),
    ('ws-no-auto-add', 'admin', 'admin@windmill.dev', true, false),
    ('ws-multi-group', 'admin', 'admin@windmill.dev', true, false)
ON CONFLICT DO NOTHING;

-- Create the 'all' group for each workspace (required for user auto-add)
INSERT INTO group_ (workspace_id, name, summary, extra_perms)
VALUES
    ('ws-with-auto-add', 'all', 'All users', '{}'),
    ('ws-no-auto-add', 'all', 'All users', '{}'),
    ('ws-multi-group', 'all', 'All users', '{}')
ON CONFLICT DO NOTHING;

-- Add admin to the 'all' group
INSERT INTO usr_to_group (workspace_id, usr, group_)
VALUES
    ('ws-with-auto-add', 'admin', 'all'),
    ('ws-no-auto-add', 'admin', 'all'),
    ('ws-multi-group', 'admin', 'all')
ON CONFLICT DO NOTHING;
