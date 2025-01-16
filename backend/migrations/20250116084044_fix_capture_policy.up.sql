-- capture config
CREATE POLICY admin_policy ON capture_config FOR ALL TO windmill_admin USING (true);
CREATE POLICY see_folder_extra_perms_user_select ON capture_config FOR SELECT TO windmill_user
USING (SPLIT_PART(capture_config.path, '/', 1) = 'f' AND SPLIT_PART(capture_config.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON capture_config FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(capture_config.path, '/', 1) = 'f' AND SPLIT_PART(capture_config.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON capture_config FOR UPDATE TO windmill_user
USING (SPLIT_PART(capture_config.path, '/', 1) = 'f' AND SPLIT_PART(capture_config.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON capture_config FOR DELETE TO windmill_user
USING (SPLIT_PART(capture_config.path, '/', 1) = 'f' AND SPLIT_PART(capture_config.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_own ON capture_config FOR ALL TO windmill_user
USING (SPLIT_PART(capture_config.path, '/', 1) = 'u' AND SPLIT_PART(capture_config.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON capture_config FOR ALL TO windmill_user
USING (SPLIT_PART(capture_config.path, '/', 1) = 'g' AND SPLIT_PART(capture_config.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));


-- capture
CREATE POLICY see_folder_extra_perms_user_select ON capture FOR SELECT TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON capture FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON capture FOR UPDATE TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON capture FOR DELETE TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_own ON capture FOR ALL TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'u' AND SPLIT_PART(capture.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON capture FOR ALL TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'g' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

