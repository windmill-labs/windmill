DROP FUNCTION acl_grants_run(JSONB, TEXT[], TEXT[]);
ALTER TABLE script DROP COLUMN view_only;
ALTER TABLE flow DROP COLUMN view_only;
ALTER TABLE resource DROP COLUMN view_only;
ALTER TABLE folder DROP COLUMN view_only;
