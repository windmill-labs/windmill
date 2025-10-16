
-- Users of instance_settings.yaml would have issues where it deletes ducklake_user_pg_pwd
-- and then the next migration fails because it tries to insert a NULL value

-- When everything is fine (i.e ducklake_user_pg_pwd or ducklake_settings is present)
-- this should be a no-op

DO $$
DECLARE
    new_settings_value text;
    old_setting_value text;
BEGIN
    SELECT value INTO new_settings_value FROM global_settings WHERE name = 'ducklake_settings';
    SELECT trim(both '"' from value::text) INTO old_setting_value FROM global_settings WHERE name = 'ducklake_user_pg_pwd';

    IF new_settings_value IS NULL AND old_setting_value IS NULL THEN
        -- Copied from 20250731132157_ducklake_instance_settings.up.sql

        INSERT INTO global_settings (name, value)
        VALUES ('ducklake_user_pg_pwd', ('"' || gen_random_uuid()::text || '"')::jsonb)
        ON CONFLICT DO NOTHING;

        -- Cannot simply create the user because Postgres expect a static string for the password
        -- Also we cannot drop the user easily in the down migration because databases will depend on it
        -- And we cannot drop databases in transactions (migrations)

        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ducklake_user') THEN
            EXECUTE format('CREATE USER ducklake_user WITH PASSWORD %L', old_setting_value);
        ELSE
            EXECUTE format('ALTER USER ducklake_user WITH PASSWORD %L', old_setting_value);
        END IF;
    END IF;
EXCEPTION
    WHEN others THEN
    RAISE NOTICE 'ducklake_user migration error, skipping.';
END
$$;