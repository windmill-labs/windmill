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
-- The schema grant matters only where USAGE was revoked from PUBLIC, in which
-- case the roles resolve no table at all and queries fail with "relation does
-- not exist". A runner that does not own the schema cannot grant it, but
-- Postgres reports that as a "no privileges were granted" warning rather than
-- an error, so it cannot abort an upgrade -- such a deployment needs
-- init-db-as-superuser.sql run by a superuser. No LOCK TABLE and no catch-all
-- handler here: every statement is idempotent and the runner owns the tables it
-- created, so the grants succeed wherever 20250205131523 was meant to.
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
