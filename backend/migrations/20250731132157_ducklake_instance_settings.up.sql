CREATE EXTENSION IF NOT EXISTS pgcrypto;

INSERT INTO global_settings (name, value)
VALUES ('ducklake_user_pg_pwd', ('"' || encode(gen_random_bytes(32), 'base64') || '"')::jsonb)
ON CONFLICT DO NOTHING;

-- Cannot simply create the user because Postgres expect a static string for the password
-- Also we cannot drop the user easily in the down migration because databases will depend on it
-- And we cannot drop databases in transactions (migrations)

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 
    FROM pg_roles 
    WHERE rolname = 'ducklake_user'
  ) THEN
    CREATE USER ducklake_user WITH PASSWORD 'TO_CHANGE';
  END IF;
EXCEPTION
  WHEN duplicate_object THEN
    RAISE NOTICE 'User ducklake_user already exists, skipping.';
END
$$;

DO $$
DECLARE
    pwd text;
BEGIN
    SELECT trim(both '"' from value::text) INTO pwd FROM global_settings WHERE name = 'ducklake_user_pg_pwd';
    EXECUTE format('ALTER USER ducklake_user WITH PASSWORD %L', pwd);
EXCEPTION
  WHEN OTHERS THEN
    RAISE NOTICE 'Couldn''t set ducklake_user password: %', SQLERRM;
END
$$;