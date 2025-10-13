INSERT INTO global_settings (name, value)
VALUES ('ducklake_user_pg_pwd', ('"' || gen_random_uuid()::text || '"')::jsonb)
ON CONFLICT DO NOTHING;

-- Cannot simply create the user because Postgres expect a static string for the password
-- Also we cannot drop the user easily in the down migration because databases will depend on it
-- And we cannot drop databases in transactions (migrations)

DO $$
DECLARE
    pwd text;
BEGIN
    SELECT trim(both '"' from value::text) INTO pwd FROM global_settings WHERE name = 'ducklake_user_pg_pwd';
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ducklake_user') THEN
        EXECUTE format('CREATE USER ducklake_user WITH PASSWORD %L', pwd);
    ELSE
      EXECUTE format('ALTER USER ducklake_user WITH PASSWORD %L', pwd);
    END IF;
EXCEPTION
  WHEN others THEN
    RAISE NOTICE 'ducklake_user migration error, skipping.';
END
$$;