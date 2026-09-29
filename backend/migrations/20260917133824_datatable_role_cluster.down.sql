-- Roles on the external cluster are live logins there; dropping the column would forget them.
LOCK TABLE datatable_role;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM datatable_role WHERE cluster <> 'instance') THEN
        RAISE EXCEPTION 'datatable_role holds roles on the external instance cluster. Delete them in instance settings first.';
    END IF;
    -- Before this, only data tables on Windmill's own cluster could be under roles, and a role
    -- block left with just `admin` survives deleting every external role.
    IF EXISTS (
        SELECT 1 FROM workspace_settings ws,
            jsonb_each(CASE WHEN jsonb_typeof(ws.datatable->'datatables') = 'object'
                THEN ws.datatable->'datatables' ELSE '{}'::jsonb END) dt
        WHERE dt.value->'database'->>'resource_type' = 'external_instance'
          AND dt.value ? 'permissions'
    ) THEN
        RAISE EXCEPTION 'external instance data tables are still under roles. Turn their roles off first.';
    END IF;
END $$;
ALTER TABLE datatable_role DROP CONSTRAINT datatable_role_cluster_name_key;
ALTER TABLE datatable_role ADD CONSTRAINT datatable_role_name_key UNIQUE (name);
ALTER TABLE datatable_role DROP COLUMN cluster;
