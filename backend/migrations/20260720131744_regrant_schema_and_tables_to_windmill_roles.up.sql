-- Re-runs the grants of 20250205131523. That migration wraps everything in a
-- single DO block that starts with LOCK TABLE pg_catalog.pg_roles, which
-- requires superuser; on managed Postgres (RDS, Cloud SQL) the migration user
-- is not one, the lock raises, and the block's EXCEPTION WHEN OTHERS handler
-- turns the failure into a NOTICE. Every GRANT after the lock is therefore
-- skipped, so core tables (workspace, script, ...) stay ungranted and any
-- statement on a user_db transaction (SET LOCAL ROLE windmill_user /
-- windmill_admin) fails with "permission denied for table" or, when the schema
-- itself was never granted, "relation does not exist". The per-table grants
-- added since (20260619091631, 20260701083047, 20260716151337) each patched one
-- table; this covers the rest.
--
-- No LOCK TABLE and no catch-all handler here: the GRANTs are idempotent, and
-- the migration user owns the tables it created, so they succeed wherever
-- 20250205131523 was meant to. A migration user that does not own the schema
-- cannot grant USAGE on it, but Postgres reports that as a "no privileges were
-- granted" warning rather than an error, so it does not abort the upgrade --
-- that deployment needs init-db-as-superuser.sql run by a superuser.
DO
$do$
DECLARE
    target_schema TEXT := current_schema();
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_user', target_schema);
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_admin', target_schema);

    EXECUTE format('GRANT ALL ON ALL TABLES IN SCHEMA %I TO windmill_user', target_schema);
    EXECUTE format('GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA %I TO windmill_user', target_schema);
    EXECUTE format('GRANT ALL ON ALL TABLES IN SCHEMA %I TO windmill_admin', target_schema);
    EXECUTE format('GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA %I TO windmill_admin', target_schema);

    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_user', target_schema);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_user', target_schema);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_admin', target_schema);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_admin', target_schema);
END
$do$;
