
DO $$
BEGIN
  ALTER ROLE custom_instance_user CREATEROLE;
EXCEPTION
    WHEN others THEN
        RAISE NOTICE 'Error in custom_instance_user migration: %', SQLERRM;
        -- Continue without failing the migration
END
$$;
