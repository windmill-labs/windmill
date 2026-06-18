-- Add up migration script here
DROP POLICY IF EXISTS admin_policy ON v2_job;
CREATE POLICY admin_policy ON v2_job FOR ALL TO windmill_admin USING (true);
