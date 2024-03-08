-- Add up migration script here
DROP POLICY see_own_path ON queue;
DROP POLICY see_member_path ON queue;
DROP POLICY see_own ON queue;
DROP POLICY see_member ON queue;
DROP POLICY see_folder_extra_perms_user ON queue;
DROP POLICY see_own_path ON completed_job;
DROP POLICY see_member_path ON completed_job;
DROP POLICY see_own ON completed_job;
DROP POLICY see_member ON completed_job;
DROP POLICY see_folder_extra_perms_user ON completed_job;


CREATE POLICY see_own_path ON queue FOR ALL
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'u' AND SPLIT_PART(queue.script_path, '/', 2) = current_setting('session.user', true));

CREATE POLICY see_member_path ON queue FOR ALL
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'g' AND SPLIT_PART(queue.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups', true), ',')::text[]));

CREATE POLICY see_own ON queue FOR ALL
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(queue.permissioned_as, '/', 2) = current_setting('session.user', true));

CREATE POLICY see_member ON queue FOR ALL
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(queue.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups', true), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user ON queue FOR ALL 
    USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text, true), ','::text)))));



CREATE POLICY see_own_path ON completed_job FOR ALL
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'u' AND SPLIT_PART(completed_job.script_path, '/', 2) = current_setting('session.user', true));

CREATE POLICY see_member_path ON completed_job FOR ALL
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'g' AND SPLIT_PART(completed_job.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups', true), ',')::text[]));

CREATE POLICY see_own ON completed_job FOR ALL
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = current_setting('session.user', true));

CREATE POLICY see_member ON completed_job FOR ALL
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups', true), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user ON completed_job FOR ALL
    USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text, true), ','::text)))));