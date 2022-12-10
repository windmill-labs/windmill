INSERT INTO workspace(id, name, owner) VALUES
	('admins', 'Admins', 'admin@windmill.dev');

INSERT INTO workspace_settings (workspace_id) VALUES
	('admins');

INSERT INTO workspace_key
	(workspace_id, kind, key)
	VALUES ('admins', 'cloud', md5(random()::text) || md5(random()::text))