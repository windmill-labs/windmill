-- Combined fixture for resource endpoint tests

-- === resource type test data ===

INSERT INTO resource_type (workspace_id, name, schema, description, created_by)
VALUES ('test-workspace', 'test_db', '{"type": "object", "properties": {"host": {"type": "string"}}}',
        'Test DB type', 'test-user');

-- === get_value_interpolated test data ===

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/simple_resource', '{"host": "localhost", "port": 5432}',
        'Simple resource', 'object', '{}', 'test-user');

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/db_password', 'hunter2', false, 'DB password', '{}');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/resource_with_var', '{"host": "localhost", "password": "$var:u/test-user/db_password"}',
        'Resource with var ref', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/db_credentials', '{"user": "admin", "password": "secret123"}',
        'DB credentials', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/resource_with_res', '{"host": "localhost", "credentials": "$res:u/test-user/db_credentials"}',
        'Resource with res ref', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/resource_mixed', '{"host": "localhost", "password": "$var:u/test-user/db_password", "credentials": "$res:u/test-user/db_credentials"}',
        'Resource with mixed refs', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/null_resource', null,
        'Null resource', 'object', '{}', 'test-user');

INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
VALUES ('test-workspace', 'u/test-user/api_key', 'sk-abc123', false, 'API key', '{}');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/inner_resource', '{"key": "$var:u/test-user/api_key"}',
        'Inner resource', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/chained_resource', '{"service": "myapi", "auth": "$res:u/test-user/inner_resource"}',
        'Chained resource', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/resource_with_array', '{"hosts": ["host1", "host2"], "port": 5432}',
        'Resource with array', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/scalar_var_resource', '"$var:u/test-user/db_password"',
        'Scalar var ref', 'string', '{}', 'test-user');

-- === fileset resource type test data ===

INSERT INTO resource_type (workspace_id, name, schema, description, created_by, is_fileset)
VALUES ('test-workspace', 'test_fileset', '{}',
        'Test fileset type', 'test-user', true);

INSERT INTO resource_type (workspace_id, name, schema, description, created_by, format_extension)
VALUES ('test-workspace', 'test_file', '{"type": "object", "properties": {"content": {"type": "string"}}}',
        'Test file type', 'test-user', 'txt');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/fileset_resource',
        '{"config.yaml": "key: value", "data/input.json": "{\"items\": []}"}',
        'A fileset resource', 'test_fileset', '{}', 'test-user');

-- === mcp_tools test data ===

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/mcp_valid', '{"name": "test-mcp", "url": "http://127.0.0.1:19999/mcp"}',
        'Valid MCP resource (unreachable)', 'mcp_server', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/mcp_invalid_format', '{"host": "localhost", "port": 5432}',
        'Not an MCP resource', 'object', '{}', 'test-user');

INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, created_by)
VALUES ('test-workspace', 'u/test-user/mcp_null', null,
        'Null MCP resource', 'mcp_server', '{}', 'test-user');
