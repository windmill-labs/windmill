-- Extends base.sql with two workspaces `test@windmill.dev` (the instance superadmin) has no
-- `usr` row in: one live, one soft-deleted. The third no-membership case, `admins`, is
-- created by migration with no `usr` rows at all and needs no fixture.

INSERT INTO workspace (id, name, owner, deleted) VALUES
	('foreign-workspace', 'foreign-workspace', 'someone-else', false),
	('archived-workspace', 'archived-workspace', 'someone-else', true);

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('foreign-workspace', 'cloud', 'test-key'),
	('archived-workspace', 'cloud', 'test-key');

INSERT INTO workspace_settings (workspace_id) VALUES
	('foreign-workspace'),
	('archived-workspace');
