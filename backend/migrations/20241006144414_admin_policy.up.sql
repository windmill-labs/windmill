-- Add up migration script here
DO
$$
DECLARE
    tbl_name text;
   	policy_exists boolean;
BEGIN
    FOR tbl_name IN
        SELECT table_name 
        FROM information_schema.tables 
        WHERE table_schema = 'public' 
          AND table_type = 'BASE TABLE'
    LOOP
        SELECT EXISTS (
            SELECT 1 
            FROM pg_policies 
            WHERE schemaname = 'public'
              AND tablename = tbl_name
              AND policyname = 'admin_policy'
        ) INTO policy_exists;

       	IF NOT policy_exists THEN
        	EXECUTE format('CREATE POLICY admin_policy ON %I TO windmill_admin USING (true);', tbl_name);
       	END IF;
    END LOOP;
END;
$$;