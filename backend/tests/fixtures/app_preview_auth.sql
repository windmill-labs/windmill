-- Fixture for the app component preview authorization regression test.
-- Layered on top of `base` (which provides test-workspace, the admin
-- `test-user`/SECRET_TOKEN, and the non-operator `test-user-2`/SECRET_TOKEN_2).
-- Adds an Operator member so we can assert that Operators cannot reach the
-- arbitrary-code app preview path (`force_viewer_static_fields` + `raw_code`).

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('operator@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Operator User');

INSERT INTO usr(workspace_id, email, username, is_admin, operator, role) VALUES
	('test-workspace', 'operator@windmill.dev', 'operator-user', false, true, 'Operator');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
	(encode(sha256('OPERATOR_TOKEN'::bytea), 'hex'), 'OPERATOR_T', 'OPERATOR_TOKEN', 'operator@windmill.dev', 'operator token', false);

-- A non-operator token scoped to `apps:run` but NOT `jobs:run`. It can reach
-- the `apps_u/execute_component` route (route maps to the `apps` scope domain)
-- but must not be able to enqueue arbitrary preview `raw_code`.
INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES
	(encode(sha256('APPS_RUN_TOKEN'::bytea), 'hex'), 'APPS_RUN_T', 'APPS_RUN_TOKEN', 'test2@windmill.dev', 'apps:run scoped token', false, '{apps:run}');

-- A private app owned by `test-user` with a persisted inline script. Used to
-- assert that `test-user-2` cannot preview-execute another app's app_script id.
INSERT INTO app (id, workspace_id, path, summary, policy, versions) VALUES
	(999001, 'test-workspace', 'u/test-user/private', 'private app', '{}'::jsonb, '{}');
INSERT INTO app_script (id, app, hash, code, code_sha256) VALUES
	(999777, 999001, repeat('a', 64), 'export function main(){ return "secret" }', repeat('b', 64));

-- An app owned by `test-user-2` with its own persisted inline script, to assert
-- the id-ownership check does not over-block a legitimate persisted preview.
INSERT INTO app (id, workspace_id, path, summary, policy, versions) VALUES
	(999002, 'test-workspace', 'u/test-user-2/ownapp', 'own app', '{}'::jsonb, '{}');
INSERT INTO app_script (id, app, hash, code, code_sha256) VALUES
	(999778, 999002, repeat('c', 64), 'export function main(){ return "ok" }', repeat('d', 64));
