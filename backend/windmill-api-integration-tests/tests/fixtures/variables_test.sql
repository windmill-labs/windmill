-- Fixture for variable endpoint tests

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/plain_var', 'hello world', false, 'A plain variable', '{}');

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/secret_var', 'supersecret', true, 'A secret variable', '{}');

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/another_var', 'foobar', false, 'Another variable', '{}');
