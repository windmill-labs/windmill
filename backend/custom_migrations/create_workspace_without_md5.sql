INSERT INTO workspace(id, name, owner) VALUES
	('admins', 'Admins', 'admin@windmill.dev') ON CONFLICT DO NOTHING;

INSERT INTO workspace_settings (workspace_id) VALUES
	('admins') ON CONFLICT DO NOTHING;

INSERT INTO workspace_key
	(workspace_id, kind, key)
	VALUES ('admins', 'cloud', array_to_string(
    array(
        SELECT chr( (trunc(65 + random() * 25)::int) + 
                    CASE WHEN random() > 0.5 THEN 32 ELSE 0 END )  -- generates random uppercase/lowercase letters
        FROM generate_series(1, 32) -- generates 32 characters
    ),
    ''
)) ON CONFLICT DO NOTHING;