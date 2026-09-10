-- Refuse while the catalog holds anything. Each row is a live Postgres login with a password
-- only this table carries, so dropping it would leave credentials on the cluster that Windmill can
-- no longer disable, delete or even name — and re-applying could not recreate them, because the
-- role names would already be taken. Cleaning them up here is not an option either: dropping a
-- role means reassigning what it owns in *every* instance database, and a migration runs in one.
--
-- Delete the roles through instance settings first; that path does the cluster work.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM datatable_role) THEN
        RAISE EXCEPTION 'Cannot roll back: % data table role(s) still exist as Postgres logins. Delete them in instance settings first, which drops them from the cluster.',
            (SELECT count(*) FROM datatable_role);
    END IF;
END $$;

DROP TABLE IF EXISTS datatable_role;
