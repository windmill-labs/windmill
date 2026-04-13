DROP POLICY IF EXISTS see_folder_extra_perms_user_insert ON http_trigger;
DROP POLICY IF EXISTS see_folder_extra_perms_user_delete ON http_trigger;
DROP POLICY IF EXISTS see_extra_perms_user_insert ON http_trigger;
DROP POLICY IF EXISTS see_extra_perms_user_delete ON http_trigger;
DROP POLICY IF EXISTS see_extra_perms_groups_insert ON http_trigger;
DROP POLICY IF EXISTS see_extra_perms_groups_delete ON http_trigger;
DROP POLICY IF EXISTS see_own ON http_trigger;
DROP POLICY IF EXISTS see_member ON http_trigger;

REVOKE INSERT, DELETE ON http_trigger FROM windmill_user;

-- Restore original prevent_route_path_change that blocks all non-admin route_path changes
CREATE OR REPLACE FUNCTION prevent_route_path_change()
RETURNS TRIGGER AS $$
BEGIN
    IF CURRENT_USER = 'windmill_user' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
