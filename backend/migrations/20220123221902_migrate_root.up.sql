DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_user') THEN

        LOCK TABLE pg_catalog.pg_roles;

        CREATE ROLE windmill_user;

    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error creating the windmill_user role: %', SQLERRM;
END
$do$;

DO
$do$
BEGIN
    LOCK TABLE pg_catalog.pg_roles;

    GRANT ALL
    ON ALL TABLES IN SCHEMA public 
    TO windmill_user;

    GRANT ALL PRIVILEGES 
    ON ALL SEQUENCES IN SCHEMA public 
    TO windmill_user;

    ALTER DEFAULT PRIVILEGES 
        IN SCHEMA public
        GRANT ALL ON TABLES TO windmill_user;

    ALTER DEFAULT PRIVILEGES 
        IN SCHEMA public
        GRANT ALL ON SEQUENCES TO windmill_user;

EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error granting proper permission to windmill_user: %', SQLERRM;
END
$do$;

DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_admin') THEN

        LOCK TABLE pg_catalog.pg_roles;

        CREATE ROLE windmill_admin WITH BYPASSRLS;


    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error creating the windmill_admin role: %', SQLERRM;
END
$do$;


DO
$do$
BEGIN
    GRANT windmill_user TO windmill_admin;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error granting windmill_user to windmill_admin: %', SQLERRM;
END
$do$;
