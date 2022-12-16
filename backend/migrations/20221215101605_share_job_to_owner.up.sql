-- Add up migration script here
ALTER TABLE queue ADD COLUMN visible_to_owner BOOLEAN DEFAULT true;
ALTER TABLE completed_job ADD COLUMN visible_to_owner BOOLEAN DEFAULT true;

CREATE POLICY see_own_path ON queue FOR ALL
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'u' AND SPLIT_PART(queue.script_path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member_path ON queue FOR ALL
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'g' AND SPLIT_PART(queue.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_own_path ON completed_job FOR ALL
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'u' AND SPLIT_PART(completed_job.script_path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member_path ON completed_job FOR ALL
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'g' AND SPLIT_PART(completed_job.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));
