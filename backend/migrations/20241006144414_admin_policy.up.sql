-- Add up migration script here
DO
$$
DECLARE
    tbl_name text;
   	policy_exists boolean;
    tbl_names text[] := ARRAY['account', 'app', 'audit', 'capture', 'completed_job', 'flow', 'folder', 'http_trigger', 'queue', 'raw_app', 'resource', 'schedule', 'script', 'usr_to_group', 'variable'];
BEGIN
    FOR tbl_name IN SELECT unnest(tbl_names)
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