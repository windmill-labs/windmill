-- A data table under roles in `test-workspace`, and a fork whose entry points at it rather than
-- carrying a copy. `test-user-2` is a non-admin of the parent and an admin of the fork: the shape
-- the pointer exists for.

INSERT INTO global_settings (name, value) VALUES
    ('custom_instance_pg_databases', '{"user_pwd": "pw", "databases": {"dt_main": {}}}'::jsonb),
    -- The role catalog has its own row: it holds generated credentials and must stay out of the
    -- operator-facing config the neighbouring row belongs to.
    ('datatable_roles', '{"role1": {"name": "analytics", "enabled": true, "pwd": "pw"}}'::jsonb)
    ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value;

UPDATE workspace_settings SET datatable = '{
    "datatables": {
        "main": {
            "database": {"resource_type": "instance", "resource_path": "dt_main"},
            "permissions": {
                "default_role": "role1",
                "roles": {
                    "admin": {"tenants": []},
                    "role1": {"tenants": ["u/test-user-2", "g/analysts", "f/finance"]}
                }
            }
        }
    }
}'::jsonb WHERE workspace_id = 'test-workspace';

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace', 'analysts', 'Analysts', '{}');
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES
	('test-workspace', 'finance', 'finance', '{}', '{}');

INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
	('wm-fork-dt', 'fork of test-workspace', 'test2@windmill.dev', 'test-workspace');
INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('wm-fork-dt', 'cloud', 'test-key');
INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('wm-fork-dt', 'all', 'All users', '{}');
INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES
	('wm-fork-dt', 'test2@windmill.dev', 'test-user-2', true, 'Admin');

INSERT INTO workspace_settings (workspace_id, datatable) VALUES ('wm-fork-dt', '{
    "datatables": {
        "main": {"reference": {"workspace_id": "test-workspace", "datatable": "main"}}
    }
}'::jsonb);
