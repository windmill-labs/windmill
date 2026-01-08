-- Revoke default privileges first
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLES FROM custom_instance_user;
REVOKE CREATE ON SCHEMA public FROM custom_instance_user;
REVOKE USAGE ON SCHEMA public FROM custom_instance_user;

DO $$
DECLARE
    dbname text := current_database();
BEGIN
    EXECUTE format('REVOKE CREATE ON DATABASE %I FROM custom_instance_user', dbname);
    EXECUTE format('REVOKE CONNECT ON DATABASE %I FROM custom_instance_user', dbname);
END $$;

