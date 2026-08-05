-- Two families for the nested dev-workspace (dev of a dev) guards.
--
-- Family A is rooted at `test-workspace` (base fixture) and is the one a nested dev is attached to:
--   test-workspace -> tw-dev ('dev')
-- plus two standalone attach candidates, one of which already owns a 'dev'-labelled dev workspace.
--
-- Family B carries a `wm-fork-` workspace re-designated as a dev workspace, which is the shape that
-- returns to being a throwaway fork on detach:
--   prod-b -> wm-fork-redev ('dev') -> redev-dev ('staging')
--
-- Family C is the ordinary prefix-less nesting. Detaching its middle workspace is fine (it returns
-- to standalone and goes on hosting `c-dev-dev`), but archiving it is not:
--   prod-c -> c-dev ('dev') -> c-dev-dev ('staging')

INSERT INTO workspace (id, name, owner, parent_workspace_id, is_dev_workspace, dev_workspace_label) VALUES
	('tw-dev', 'dev of test-workspace', 'test@windmill.dev', 'test-workspace', true, 'dev'),
	('standalone', 'standalone', 'test@windmill.dev', NULL, false, NULL),
	('standalone-dev', 'dev of standalone', 'test@windmill.dev', 'standalone', true, 'dev'),
	('spare', 'spare standalone', 'test@windmill.dev', NULL, false, NULL),
	('prod-b', 'prod b', 'test@windmill.dev', NULL, false, NULL),
	('wm-fork-redev', 'redesignated fork', 'test@windmill.dev', 'prod-b', true, 'dev'),
	('redev-dev', 'dev of the redesignated fork', 'test@windmill.dev', 'wm-fork-redev', true, 'staging'),
	('prod-c', 'prod c', 'test@windmill.dev', NULL, false, NULL),
	('c-dev', 'dev of prod-c', 'test@windmill.dev', 'prod-c', true, 'dev'),
	('c-dev-dev', 'dev of c-dev', 'test@windmill.dev', 'c-dev', true, 'staging');

CREATE TEMP VIEW new_workspaces AS SELECT unnest(ARRAY[
	'tw-dev', 'standalone', 'standalone-dev', 'spare', 'prod-b', 'wm-fork-redev', 'redev-dev',
	'prod-c', 'c-dev', 'c-dev-dev'
]) AS id;

INSERT INTO workspace_settings (workspace_id)
	SELECT id FROM new_workspaces;

INSERT INTO workspace_key (workspace_id, kind, key)
	SELECT id, 'cloud', 'test-key' FROM new_workspaces;

INSERT INTO group_ (workspace_id, name, summary, extra_perms)
	SELECT id, 'all', 'All users', '{}' FROM new_workspaces;

INSERT INTO usr (workspace_id, email, username, is_admin, role)
	SELECT id, 'test@windmill.dev', 'test-user', true, 'Admin'
	FROM new_workspaces;

DROP VIEW new_workspaces;
