-- Fixture for the inline preview authorization regression test (GHSA-pp5h-96x3-3wqq).
-- Layered on top of `base` (which provides test-workspace and the non-operator
-- `test-user-2`/SECRET_TOKEN_2). Adds an Operator member so we can assert that
-- Operators cannot reach the arbitrary-code inline preview path
-- (`POST /jobs/run_inline/preview`) with their own token, plus two deployed script
-- jobs of the operator: one running, so we can assert that its WM_TOKEN can, and
-- one queued but not yet pulled, so we can assert that "queued" is not enough.

INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
    VALUES ('operator@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Operator User');

INSERT INTO usr(workspace_id, email, username, is_admin, operator, role) VALUES
	('test-workspace', 'operator@windmill.dev', 'operator-user', false, true, 'Operator');

INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin) VALUES
	(encode(sha256('OPERATOR_TOKEN'::bytea), 'hex'), 'OPERATOR_T', 'OPERATOR_TOKEN', 'operator@windmill.dev', 'operator token', false);

INSERT INTO v2_job(id, workspace_id, kind, runnable_path, created_by, permissioned_as, permissioned_as_email) VALUES
	('2aa0c0de-0000-4000-8000-000000000001', 'test-workspace', 'script', 'u/test-user/deployed', 'operator-user', 'u/operator-user', 'operator@windmill.dev'),
	('2aa0c0de-0000-4000-8000-000000000002', 'test-workspace', 'script', 'u/test-user/deployed', 'operator-user', 'u/operator-user', 'operator@windmill.dev');

INSERT INTO v2_job_queue(id, workspace_id, scheduled_for, running) VALUES
	('2aa0c0de-0000-4000-8000-000000000001', 'test-workspace', now(), true),
	('2aa0c0de-0000-4000-8000-000000000002', 'test-workspace', now(), false);
