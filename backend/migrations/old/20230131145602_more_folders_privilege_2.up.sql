-- Add up migration script here

CREATE POLICY see_folder_extra_perms_user ON account FOR ALL
USING (SPLIT_PART(account.owner, '/', 1) = 'f' AND SPLIT_PART(account.owner, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
WITH CHECK (SPLIT_PART(account.owner, '/', 1) = 'f' AND SPLIT_PART(account.owner, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
