DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
        ALTER ROLE custom_instance_user REPLICATION;
    END IF;

    DROP ROLE IF EXISTS custom_instance_replication_user;

    DELETE FROM global_settings WHERE name = 'custom_instance_replication_pwd';
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'custom_instance_replication_user down-migration error, skipping: %', SQLERRM;
END
$$;
