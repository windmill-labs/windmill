-- Add missing INSERT and DELETE RLS policies for http_trigger table.
-- These are needed now that non-admin users can create HTTP triggers (with forced workspaced routes).
-- All other trigger tables already have these policies.

-- Folder-based policies
CREATE POLICY see_folder_extra_perms_user_insert ON http_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(http_trigger.path, '/', 1) = 'f' AND SPLIT_PART(http_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user_delete ON http_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(http_trigger.path, '/', 1) = 'f' AND SPLIT_PART(http_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

-- User extra_perms policies
CREATE POLICY see_extra_perms_user_insert ON http_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_user_delete ON http_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

-- Group extra_perms policies
CREATE POLICY see_extra_perms_groups_insert ON http_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

CREATE POLICY see_extra_perms_groups_delete ON http_trigger FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

-- Owner and member policies (other trigger tables have these, http_trigger was missing them)
CREATE POLICY see_own ON http_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(http_trigger.path, '/', 1) = 'u' AND SPLIT_PART(http_trigger.path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON http_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(http_trigger.path, '/', 1) = 'g' AND SPLIT_PART(http_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));
