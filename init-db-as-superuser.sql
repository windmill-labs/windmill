DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_user') THEN

        LOCK TABLE pg_catalog.pg_roles;

        CREATE ROLE windmill_user;

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

    END IF;
END
$do$;

DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_admin') THEN
        CREATE ROLE windmill_admin WITH BYPASSRLS;

        GRANT ALL
        ON ALL TABLES IN SCHEMA public 
        TO windmill_admin;

        GRANT ALL PRIVILEGES 
        ON ALL SEQUENCES IN SCHEMA public 
        TO windmill_admin;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON TABLES TO windmill_admin;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON SEQUENCES TO windmill_admin;
    END IF;
END
$do$;
