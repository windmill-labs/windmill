-- Every definer-rights function (prosecdef) owned by the migration role gets a search_path
-- ending in pg_temp, so a session's temporary objects never resolve ahead of the schema's
-- (pg_temp is searched first unless the path names it), and loses the default PUBLIC EXECUTE
-- grant, so only Windmill's own roles use it. The schema comes from current_schema() so installs
-- with a non-public PG_SCHEMA resolve (see 20260624103600). A later CREATE OR REPLACE resets the
-- path and a new function starts with the PUBLIC grant: both need the same two statements.
DO
$do$
DECLARE
    target_schema TEXT := current_schema();
    fn regprocedure;
BEGIN
    FOR fn IN
        SELECT p.oid::regprocedure
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = target_schema
          AND p.prosecdef
          AND pg_has_role(current_user, p.proowner, 'USAGE')
    LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', fn, target_schema);
        EXECUTE format('REVOKE EXECUTE ON FUNCTION %s FROM PUBLIC', fn);
    END LOOP;
END
$do$;

-- folder_labels() is the only one that is called rather than fired, from user_db transactions
-- (SET LOCAL ROLE windmill_user / windmill_admin) and by the owner. Trigger functions need no
-- grant: EXECUTE is checked when a trigger is created, not when it fires.
GRANT EXECUTE ON FUNCTION folder_labels(text, text) TO windmill_user, windmill_admin;
