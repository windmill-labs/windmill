DO
$do$
BEGIN

    CREATE ROLE windmill_user;

EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error creating the windmill_user role: %', SQLERRM;
END
$do$;

DO
$do$
BEGIN

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

    CREATE ROLE windmill_admin WITH BYPASSRLS;

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
