-- for some managed dbs (e.g Azure Postgres) the role is not 
-- automatically applied to the user when created
DO
$do$
BEGIN
    GRANT windmill_user to CURRENT_USER;
    GRANT windmill_admin to CURRENT_USER;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'error granting windmill_user and windmill_admin to current_user: %', SQLERRM;
END
$do$;

