-- Fixture for testing wmill CLI variable/resource get from bash scripts

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/test_var', 'hello from variable', false, 'A test variable', '{"u/test-user": true}');

INSERT INTO resource_type (workspace_id, name, schema, description, created_by)
VALUES ('test-workspace', 'test_object', '{}', 'Test object type', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/test_res', '{"host": "localhost", "port": 5432}', 'A test resource', 'test_object', '{"u/test-user": true}', 'test-user');
