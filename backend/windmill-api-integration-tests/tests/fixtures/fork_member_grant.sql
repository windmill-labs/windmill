-- `test2` (a non-admin developer of test-workspace) has forked it. The fork's `usr` row copies the
-- non-admin role they hold in the parent, which is the situation the fork-creator grant exists for.
INSERT INTO workspace (id, name, owner, parent_workspace_id) VALUES
	('wm-fork-test', 'fork of test-workspace', 'test2@windmill.dev', 'test-workspace');

INSERT INTO workspace_settings (workspace_id) VALUES ('wm-fork-test');

INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('wm-fork-test', 'cloud', 'test-key');

INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('wm-fork-test', 'all', 'All users', '{}');

INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES
	('wm-fork-test', 'test2@windmill.dev', 'test-user-2', false, 'User'),
	-- An admin of the fork, whom its creator must not be able to remove.
	('wm-fork-test', 'test@windmill.dev', 'test-user', true, 'Admin');

-- An operator of the parent: the eligibility bar for being added to the fork is developer-or-above
-- there, so this user must be rejected.
INSERT INTO password (email, password_hash, login_type, super_admin, verified, name, username)
	VALUES ('test4@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Test User 4', 'test-user-4');

INSERT INTO usr (workspace_id, email, username, is_admin, operator, role) VALUES
	('test-workspace', 'test4@windmill.dev', 'test-user-4', false, true, 'Operator');

-- add_user resolves the instance-wide username from `password`.
UPDATE password SET username = 'test-user-2' WHERE email = 'test2@windmill.dev';
UPDATE password SET username = 'test-user-3' WHERE email = 'test3@windmill.dev';

-- With automated username creation off, `add_user` takes the username from the caller. That branch
-- is what the fork creator must not be able to steer, so the tests run against it.
INSERT INTO global_settings (name, value) VALUES ('automate_username_creation', 'false'::jsonb)
	ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value;
