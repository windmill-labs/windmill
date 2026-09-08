-- Dedicated logical-replication role used by postgres triggers on custom-instance
-- datatables. Its password is stored server-only in global_settings.custom_instance_replication_pwd
-- (hidden from the config surface); membership in custom_instance_user lets it manage
-- publications on the datatable tables. custom_instance_user itself must not hold REPLICATION.
DO $$
DECLARE
    pwd text;
BEGIN
    SELECT gen_random_uuid()::text INTO pwd;

    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_replication_user') THEN
        EXECUTE format('ALTER USER custom_instance_replication_user WITH PASSWORD %L REPLICATION', pwd);
    ELSE
        EXECUTE format('CREATE USER custom_instance_replication_user WITH PASSWORD %L REPLICATION', pwd);
    END IF;

    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
        GRANT custom_instance_user TO custom_instance_replication_user;
        ALTER ROLE custom_instance_user NOREPLICATION;
    END IF;

    INSERT INTO global_settings (name, value)
    VALUES ('custom_instance_replication_pwd', to_jsonb(pwd::text))
    ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value;

    -- Drop any replication password an earlier iteration stored in the operator-facing row.
    UPDATE global_settings
    SET value = value - 'replication_user_pwd'
    WHERE name = 'custom_instance_pg_databases';
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'custom_instance_replication_user migration error, skipping: %', SQLERRM;
END
$$;
