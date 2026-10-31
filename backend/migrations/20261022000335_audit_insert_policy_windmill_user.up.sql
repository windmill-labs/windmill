-- Only the server writes audit rows, so insert policies guard nothing. They did reject rows
-- whose username is not the caller, such as a job pushed as another user by a non-admin, and
-- that failed insert aborted the caller's transaction. Reads stay restricted by `see_own`.
DROP POLICY IF EXISTS schedule ON audit_partitioned;
DROP POLICY IF EXISTS schedule_audit ON audit_partitioned;
DROP POLICY IF EXISTS webhook ON audit_partitioned;
CREATE POLICY insert_any ON audit_partitioned FOR INSERT TO windmill_user WITH CHECK (true);
