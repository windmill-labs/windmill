-- Re-runs the grants of 20250205131523. Everything in that migration sits in
-- one DO block whose first statement is LOCK TABLE pg_catalog.pg_roles, which
-- a non-superuser cannot take; on managed Postgres (RDS, Cloud SQL) it raises
-- and the block's single EXCEPTION WHEN OTHERS handler downgrades the failure
-- to a NOTICE, so every GRANT after it is skipped.
--
-- Most tables survive that anyway, because 20221105003256 grants them outside
-- any such block. What is lost is the ALTER DEFAULT PRIVILEGES, which is what
-- covers tables created by later migrations. Those default privileges only
-- apply to objects created by the role that set them, so a deployment whose
-- migration runner never successfully ran them ends up with newer tables
-- ungranted and writes on a user_db transaction (SET LOCAL ROLE windmill_user /
-- windmill_admin) failing with "permission denied for table". That gap is why
-- 20260619091631, 20260701083047 and 20260716151337 each had to patch one
-- table by hand; re-establishing the default privileges under the current
-- runner closes it for future tables instead.
--
-- This migration must never abort an upgrade, so every grant is guarded
-- individually. That is the opposite of 20250205131523's single block-wide
-- WHEN OTHERS, whose flaw was granularity, not the mere presence of a handler:
-- one early failure there silently skipped every remaining grant. Here each
-- grant that cannot be applied is isolated and re-raised as a named WARNING, so
-- the rest still run and the reason is visible in the migration log. The known
-- failure modes this absorbs:
--   * an object the runner cannot grant on -- a superuser-installed extension
--     (PostGIS's spatial_ref_sys) or a co-located application table owned by an
--     unrelated role. The loops filter to relations the runner has authority to
--     grant, so these are skipped without even a warning; the guard is the
--     backstop.
--   * an object dropped by another session between the catalog scan and the
--     GRANT (only possible when something outside the migration shares the DB).
--   * USAGE on a schema the runner does not own (needed only where USAGE was
--     revoked from PUBLIC, else the roles resolve no table and queries fail
--     with "relation does not exist"); such a deployment needs
--     init-db-as-superuser.sql run by a superuser.
-- windmill_user and windmill_admin are guaranteed to exist: 20221105003256
-- grants to both outside any handler, so any database that reached this
-- migration already has them.
DO
$do$
DECLARE
    target_schema TEXT := current_schema();
    obj TEXT;
BEGIN
    BEGIN
        EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_user, windmill_admin', target_schema);
    EXCEPTION WHEN OTHERS THEN
        RAISE WARNING 'skipped GRANT USAGE on schema %: %', target_schema, SQLERRM;
    END;

    -- pg_class over the relkinds GRANT ... ON ALL TABLES covers -- ordinary (r),
    -- partitioned (p), views (v), materialized views (m), foreign (f). pg_tables
    -- would miss views such as flow_workspace_runnables, which are read through
    -- user_db transactions and so must be granted too. pg_has_role(...,'USAGE')
    -- selects the relations the runner can actually grant: ones it owns
    -- directly, inherits ownership of (tables still owned by a previous
    -- migration role after a credential rotation), or reaches as a superuser.
    -- Owner-name equality would miss the inherited-owner case and leave those
    -- relations' user_db access broken.
    FOR obj IN
        SELECT format('%I.%I', n.nspname, c.relname)
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = target_schema
          AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
          AND pg_has_role(current_user, c.relowner, 'USAGE')
    LOOP
        BEGIN
            EXECUTE format('GRANT ALL ON TABLE %s TO windmill_user, windmill_admin', obj);
        EXCEPTION WHEN OTHERS THEN
            RAISE WARNING 'skipped GRANT on relation %: %', obj, SQLERRM;
        END;
    END LOOP;

    FOR obj IN
        SELECT format('%I.%I', schemaname, sequencename)
        FROM pg_sequences
        WHERE schemaname = target_schema
          AND pg_has_role(current_user, sequenceowner, 'USAGE')
    LOOP
        BEGIN
            EXECUTE format('GRANT ALL ON SEQUENCE %s TO windmill_user, windmill_admin', obj);
        EXCEPTION WHEN OTHERS THEN
            RAISE WARNING 'skipped GRANT on sequence %: %', obj, SQLERRM;
        END;
    END LOOP;

    -- Applies to future objects created by the runner (the FOR ROLE default),
    -- so it cannot conflict with objects owned by anyone else.
    BEGIN
        EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_user, windmill_admin', target_schema);
        EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_user, windmill_admin', target_schema);
    EXCEPTION WHEN OTHERS THEN
        RAISE WARNING 'skipped ALTER DEFAULT PRIVILEGES in schema %: %', target_schema, SQLERRM;
    END;
END
$do$;
