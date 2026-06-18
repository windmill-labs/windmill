DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
    ALTER ROLE custom_instance_user NOREPLICATION;
  END IF;
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'Error revoking REPLICATION from custom_instance_user: %', SQLERRM;
END
$$;
