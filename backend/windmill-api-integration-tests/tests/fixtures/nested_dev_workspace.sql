-- Seven families for the nested dev-workspace (dev of a dev) guards.
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
--
-- Family E has no nested dev yet, so both "give `wm-fork-edev` a dev" and "detach `wm-fork-edev`"
-- pass their own checks — the pair that must not both commit:
--   prod-e -> wm-fork-edev ('dev'), plus the standalone candidate `e-cand`
--
-- Family F is three standalone workspaces, so "attach f-mid under prod-f" and "attach f-leaf under
-- f-mid" both pass on their own — adjacent attaches whose labels only collide once both land:
--   prod-f, f-mid, f-leaf
--
-- Family G already nests, so two attaches at opposite ends of it touch no workspace in common —
-- their labels only collide once both land, three dev workspaces deep:
--   prod-g (root), g-mid -> g-sub ('dev'), and the standalone `g-leaf`
--
-- Family H is a standalone that already owns a dev: archiving it resolves as "no pairing involved"
-- while still being an operation the pairing lock has to cover:
--   h-cand -> h-sub ('dev')


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
	('c-dev-dev', 'dev of c-dev', 'test@windmill.dev', 'c-dev', true, 'staging'),
	('prod-e', 'prod e', 'test@windmill.dev', NULL, false, NULL),
	('wm-fork-edev', 'redesignated fork with no dev yet', 'test@windmill.dev', 'prod-e', true, 'dev'),
	('e-cand', 'attach candidate', 'test@windmill.dev', NULL, false, NULL),
	('prod-f', 'prod f', 'test@windmill.dev', NULL, false, NULL),
	('f-mid', 'middle attach candidate', 'test@windmill.dev', NULL, false, NULL),
	('f-leaf', 'leaf attach candidate', 'test@windmill.dev', NULL, false, NULL),
	('prod-g', 'prod g', 'test@windmill.dev', NULL, false, NULL),
	('g-mid', 'standalone with a dev of its own', 'test@windmill.dev', NULL, false, NULL),
	('g-sub', 'dev of g-mid', 'test@windmill.dev', 'g-mid', true, 'dev'),
	('g-leaf', 'leaf attach candidate', 'test@windmill.dev', NULL, false, NULL),
	('h-cand', 'standalone owning a dev', 'test@windmill.dev', NULL, false, NULL),
	('h-sub', 'dev of h-cand', 'test@windmill.dev', 'h-cand', true, 'dev');

CREATE TEMP VIEW new_workspaces AS SELECT unnest(ARRAY[
	'tw-dev', 'standalone', 'standalone-dev', 'spare', 'prod-b', 'wm-fork-redev', 'redev-dev',
	'prod-c', 'c-dev', 'c-dev-dev', 'prod-e', 'wm-fork-edev', 'e-cand',
	'prod-f', 'f-mid', 'f-leaf', 'prod-g', 'g-mid', 'g-sub', 'g-leaf',
	'h-cand', 'h-sub'
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
