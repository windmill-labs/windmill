DO
$do$
DECLARE
    current_schema_name TEXT;
BEGIN
    -- Get the current schema for the session
    SELECT current_schema() INTO current_schema_name;

    -- Lock the roles table to prevent race conditions
    LOCK TABLE pg_catalog.pg_roles;



    EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_user', current_schema_name);
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO windmill_admin', current_schema_name);


    -- Grant privileges dynamically to the current schema
    EXECUTE format('GRANT ALL ON ALL TABLES IN SCHEMA %I TO windmill_user', current_schema_name);
    EXECUTE format('GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA %I TO windmill_user', current_schema_name);

    -- Alter default privileges dynamically
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_user', current_schema_name);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_user', current_schema_name);

        -- Grant privileges dynamically to the current schema
    EXECUTE format('GRANT ALL ON ALL TABLES IN SCHEMA %I TO windmill_admin', current_schema_name);
    EXECUTE format('GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA %I TO windmill_admin', current_schema_name);

    -- Alter default privileges dynamically
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO windmill_admin', current_schema_name);
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON SEQUENCES TO windmill_admin', current_schema_name);

EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Error granting proper permissions to windmill users: %', SQLERRM;
END
$do$;
