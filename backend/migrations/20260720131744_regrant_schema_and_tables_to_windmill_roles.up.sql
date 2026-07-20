-- Re-runs the grants of 20250205131523. Everything in that migration sits in
-- one DO block whose first statement is LOCK TABLE pg_catalog.pg_roles, which
-- a non-superuser cannot take; on managed Postgres (RDS, Cloud SQL) it raises
-- and the block's EXCEPTION WHEN OTHERS handler downgrades the failure to a
-- NOTICE, so every GRANT after it is skipped.
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
-- We grant per-object over only the tables and sequences the runner owns,
-- instead of GRANT ... ON ALL TABLES IN SCHEMA. The blanket form raises a hard
-- "permission denied for table X" -- aborting the whole upgrade, since there is
-- deliberately no catch-all handler -- the moment the schema holds one object
-- the runner does not own, e.g. a superuser-installed extension (PostGIS's
-- spatial_ref_sys) or a co-located application table. 20250205131523 tolerated
-- that only by swallowing every error. Owning the object is exactly the
-- condition under which the GRANT can succeed, so filtering to owned objects
-- skips what would fail (Windmill's own tables are all owned by the runner)
-- rather than hiding it. windmill_user and windmill_admin are guaranteed to
-- exist here: 20221105003256 grants to both outside any handler, so any
-- database that reached this migration already has them.
DO
$do$
DECLARE
    target_schema TEXT := current_schema();
    obj TEXT;
BEGIN
    -- USAGE on the schema, needed only where it was revoked from PUBLIC. A
    -- runner that does not own the schema cannot grant it, but Postgres reports
    -- that as a "no privileges were granted" warning, not an error, so it
    -- cannot abort the upgrade; that deployment needs init-db-as-superuser.sql
    -- run by a superuser.
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_user, windmill_admin', target_schema);

    FOR obj IN
        SELECT format('%I.%I', schemaname, tablename)
        FROM pg_tables
        WHERE schemaname = target_schema AND tableowner = current_user
    LOOP
        EXECUTE format('GRANT ALL ON TABLE %s TO windmill_user, windmill_admin', obj);
    END LOOP;

    FOR obj IN
        SELECT format('%I.%I', schemaname, sequencename)
        FROM pg_sequences
        WHERE schemaname = target_schema AND sequenceowner = current_user
    LOOP
        EXECUTE format('GRANT ALL ON SEQUENCE %s TO windmill_user, windmill_admin', obj);
    END LOOP;

    -- Applies to future objects created by the runner (the FOR ROLE default),
    -- so it cannot conflict with objects owned by anyone else.
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_user, windmill_admin', target_schema);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_user, windmill_admin', target_schema);
END
$do$;
