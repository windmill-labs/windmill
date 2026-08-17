-- Fixture for the inline preview authorization regression test (GHSA-pp5h-96x3-3wqq).
-- Layered on top of `base` (which provides test-workspace and the non-operator
-- `test-user-2`/SECRET_TOKEN_2). Adds an Operator member so we can assert that
-- Operators cannot reach the arbitrary-code inline preview path
-- (`POST /jobs/run_inline/preview`).

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('operator@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Operator User');

INSERT INTO usr(workspace_id, email, username, is_admin, operator, role) VALUES
	('test-workspace', 'operator@windmill.dev', 'operator-user', false, true, 'Operator');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
	(encode(sha256('OPERATOR_TOKEN'::bytea), 'hex'), 'OPERATOR_T', 'OPERATOR_TOKEN', 'operator@windmill.dev', 'operator token', false);
