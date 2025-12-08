DO $$
DECLARE
    pwd text;
BEGIN
    SELECT gen_random_uuid()::text INTO pwd;
    
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
        EXECUTE format('ALTER USER custom_instance_user WITH PASSWORD %L', pwd);
        RAISE NOTICE 'Updated password for existing user custom_instance_user';
    ELSE
        EXECUTE format('CREATE USER custom_instance_user WITH PASSWORD %L', pwd);
        RAISE NOTICE 'Created new user custom_instance_user';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM global_settings WHERE name = 'custom_instance_pg_databases') THEN
        INSERT INTO global_settings (name, value)
        VALUES ('custom_instance_pg_databases', jsonb_build_object(
          'user_pwd', pwd::text,
          'databases', jsonb_build_object()
        ));
        RAISE NOTICE 'Inserted new global setting for custom_instance_pg_databases';
    ELSE
      UPDATE global_settings
      SET value = jsonb_set(COALESCE(value, '{}'::jsonb), '{user_pwd}', to_jsonb(pwd::text)::jsonb)
      WHERE name = 'custom_instance_pg_databases';
      RAISE NOTICE 'Updated user_pwd in existing global setting for custom_instance_pg_databases';
    END IF;
EXCEPTION
    WHEN others THEN
    RAISE NOTICE 'custom_instance_pg_databases migration error, skipping.';
END
$$;
