-- Roles on the external cluster are live logins there; dropping the column would forget them.
LOCK TABLE datatable_role;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM datatable_role WHERE cluster <> 'instance') THEN
        RAISE EXCEPTION 'datatable_role holds roles on the external instance cluster. Delete them in instance settings first.';
    END IF;
END $$;
ALTER TABLE datatable_role DROP CONSTRAINT datatable_role_cluster_name_key;
ALTER TABLE datatable_role ADD CONSTRAINT datatable_role_name_key UNIQUE (name);
ALTER TABLE datatable_role DROP COLUMN cluster;
