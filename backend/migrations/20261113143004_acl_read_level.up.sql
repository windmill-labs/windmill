-- A grant in `extra_perms` is `true` (write), `false` (the level below write: run where the
-- item runs, read elsewhere) or one of "read", "run", "write". These policies read it without
-- the boolean cast they had, which a string value would make fail for every writer of the row.
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['script', 'flow', 'resource'] LOOP
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_insert ON %1$I WITH CHECK (
            (extra_perms ->> (SELECT concat('u/', current_setting('session.user'))))
                IN ('true', 'write'))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_update ON %1$I USING (
            (extra_perms ->> (SELECT concat('u/', current_setting('session.user'))))
                IN ('true', 'write'))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_delete ON %1$I USING (
            (extra_perms ->> (SELECT concat('u/', current_setting('session.user'))))
                IN ('true', 'write'))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_insert ON %1$I WITH CHECK (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value IN ('true', 'write')))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_update ON %1$I USING (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value IN ('true', 'write')))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_delete ON %1$I USING (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value IN ('true', 'write')))$p$, t);
    END LOOP;
END $$;

-- Whether any of `principals` holds a grant in `extra_perms` of a runnable that includes running.
CREATE FUNCTION acl_grants_run(extra_perms JSONB, principals TEXT[])
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM jsonb_each_text(extra_perms) e
        WHERE e.key = ANY(principals) AND e.value <> 'read'
    )
$$;
