DO $$
DECLARE
    dbname text := current_database();
BEGIN
    -- Revoke default privileges first
    ALTER DEFAULT PRIVILEGES IN SCHEMA public
        REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLES FROM custom_instance_user;
    REVOKE CREATE ON SCHEMA public FROM custom_instance_user;
    REVOKE USAGE ON SCHEMA public FROM custom_instance_user;
    EXECUTE format('REVOKE CREATE ON DATABASE %I FROM custom_instance_user', dbname);
    EXECUTE format('REVOKE CONNECT ON DATABASE %I FROM custom_instance_user', dbname);
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'Error in custom_instance_user migration: %', SQLERRM;
        -- Continue without failing the migration
END
$$;
