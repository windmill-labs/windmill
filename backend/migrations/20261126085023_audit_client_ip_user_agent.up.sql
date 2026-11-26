ALTER TABLE audit_partitioned ADD COLUMN IF NOT EXISTS client_ip TEXT, ADD COLUMN IF NOT EXISTS user_agent TEXT;

-- Older servers read `SELECT * FROM audit_partitioned UNION ALL SELECT * FROM audit`, so the
-- legacy table, or the empty view that replaces it, carries the same columns in the same order.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_class WHERE oid = to_regclass('audit') AND relkind = 'r') THEN
        ALTER TABLE audit ADD COLUMN IF NOT EXISTS client_ip TEXT, ADD COLUMN IF NOT EXISTS user_agent TEXT;
    ELSIF EXISTS (SELECT 1 FROM pg_class WHERE oid = to_regclass('audit') AND relkind = 'v') THEN
        CREATE OR REPLACE VIEW audit AS
            SELECT workspace_id, id, timestamp, username, operation, action_kind, resource,
                   parameters, email, span, client_ip, user_agent
            FROM audit_partitioned WHERE false;
    END IF;
END $$;
