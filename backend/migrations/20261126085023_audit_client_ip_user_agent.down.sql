DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_class WHERE oid = to_regclass('audit') AND relkind = 'r') THEN
        ALTER TABLE audit DROP COLUMN IF EXISTS client_ip, DROP COLUMN IF EXISTS user_agent;
    ELSIF EXISTS (SELECT 1 FROM pg_class WHERE oid = to_regclass('audit') AND relkind = 'v') THEN
        DROP VIEW audit;
        CREATE VIEW audit AS
            SELECT workspace_id, id, timestamp, username, operation, action_kind, resource,
                   parameters, email, span
            FROM audit_partitioned WHERE false;
        GRANT ALL ON audit TO windmill_user, windmill_admin;
    END IF;
END $$;

ALTER TABLE audit_partitioned DROP COLUMN IF EXISTS client_ip, DROP COLUMN IF EXISTS user_agent;
