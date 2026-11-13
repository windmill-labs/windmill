-- Read access to a script or flow shared item by item (its extra_perms) shows that item's runs,
-- as read access to its folder already does.
--
-- The shared items are gathered once per statement (an InitPlan) into a jsonb object and each
-- row is a key lookup. Do not turn this into `runnable_path = ANY (array)`: that compares every
-- scanned row against the whole list, and a user whose shared items rarely run makes the runs
-- list scan the whole table (22 s at 1,000 shared items on 3M jobs, against 1.4 s with the lookup).
-- Keys carry the workspace because a username only means something inside one workspace.
CREATE POLICY see_item_extra_perms ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING (
        visible_to_owner IS TRUE
        AND (
            SELECT coalesce(jsonb_object_agg(shared.key, true), '{}'::jsonb)
            FROM (
                SELECT workspace_id || ':' || path AS key
                FROM script
                WHERE NOT archived AND NOT deleted
                  AND (extra_perms ? (SELECT concat('u/', current_setting('session.user')))
                       OR extra_perms ?| (SELECT regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
                UNION
                SELECT workspace_id || ':' || path
                FROM flow
                WHERE NOT archived
                  AND (extra_perms ? (SELECT concat('u/', current_setting('session.user')))
                       OR extra_perms ?| (SELECT regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
            ) shared
        ) ? (workspace_id || ':' || runnable_path)
    );
