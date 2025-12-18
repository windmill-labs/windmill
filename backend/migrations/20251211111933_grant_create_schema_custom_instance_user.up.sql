-- Grant CREATE privilege on all databases where custom_instance_user has CONNECT
-- This allows custom_instance_user to create schemas in databases it can already access

DO $$
DECLARE
    db_record RECORD;
    grant_command TEXT;
BEGIN
    -- Find all databases where custom_instance_user has CONNECT privilege
    -- We check if the datacl array contains an entry for custom_instance_user with 'c' (CONNECT) privilege
    FOR db_record IN
        SELECT d.datname
        FROM pg_database d
        WHERE d.datname NOT IN ('template0', 'template1')  -- Skip template databases
        AND d.datallowconn = true  -- Only consider databases that allow connections
        AND d.datacl IS NOT NULL  -- Has ACL entries
        AND EXISTS (
            SELECT 1
            FROM unnest(d.datacl) AS acl_entry
            WHERE acl_entry::text LIKE 'custom_instance_user=c/%'  -- 'c' is the privilege code for CONNECT
        )
    LOOP
        BEGIN
            -- Grant CREATE privilege on the database
            EXECUTE format('GRANT CREATE ON DATABASE %I TO custom_instance_user', db_record.datname);
            RAISE NOTICE 'Granted CREATE on database % to custom_instance_user', db_record.datname;
        EXCEPTION
            WHEN others THEN
                RAISE NOTICE 'Failed to grant CREATE on database %: %', db_record.datname, SQLERRM;
        END;
    END LOOP;
    RAISE NOTICE 'Completed granting CREATE privileges to custom_instance_user on all accessible databases';
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'Error in custom_instance_user CREATE privilege migration: %', SQLERRM;
        -- Continue without failing the migration
END
$$;
