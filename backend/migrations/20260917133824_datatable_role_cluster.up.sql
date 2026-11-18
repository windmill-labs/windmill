-- A data table role is a Postgres login on one cluster: Windmill's own ('instance'), or the external
-- instance cluster ('external_instance'). Role names are the cluster's own key, so they are unique
-- per cluster rather than across the instance.
ALTER TABLE datatable_role
    ADD COLUMN cluster VARCHAR(20) NOT NULL DEFAULT 'instance'
        CHECK (cluster IN ('instance', 'external_instance'));
ALTER TABLE datatable_role DROP CONSTRAINT datatable_role_name_key;
ALTER TABLE datatable_role ADD CONSTRAINT datatable_role_cluster_name_key UNIQUE (cluster, name);
