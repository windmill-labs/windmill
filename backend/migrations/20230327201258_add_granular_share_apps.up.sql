-- Add up migration script here
CREATE POLICY see_extra_perms_user ON app FOR ALL
USING (extra_perms ? CONCAT('u/', current_setting('session.user')))
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups ON app FOR ALL
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));