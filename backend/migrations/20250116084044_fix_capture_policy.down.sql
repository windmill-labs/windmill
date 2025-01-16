-- Add down migration script here
DROP POLICY admin_policy ON capture_config;
DROP POLICY see_folder_extra_perms_user_select ON capture_config;
DROP POLICY see_folder_extra_perms_user_insert ON capture_config;
DROP POLICY see_folder_extra_perms_user_update ON capture_config;
DROP POLICY see_folder_extra_perms_user_delete ON capture_config;
DROP POLICY see_own ON capture_config;
DROP POLICY see_member ON capture_config;


DROP POLICY see_folder_extra_perms_user_select ON capture;
DROP POLICY see_folder_extra_perms_user_insert ON capture;
DROP POLICY see_folder_extra_perms_user_update ON capture;
DROP POLICY see_folder_extra_perms_user_delete ON capture;
DROP POLICY see_own ON capture;
DROP POLICY see_member ON capture;
