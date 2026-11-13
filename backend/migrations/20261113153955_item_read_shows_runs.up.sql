-- Read access to a script or flow shared item by item (its extra_perms) shows that item's runs,
-- as read access to its folder already does.

-- The `<kind>:<workspace>:<path>` keys of the scripts and flows shared with the session's user
-- or one of their groups. The kind is part of the key because a script and a flow can share a
-- path, each with its own sharing. SECURITY DEFINER: run as windmill_user, the row policies of script and flow
-- would apply inside, and their non-leakproof jsonb checks keep the extra_perms GIN indexes
-- from being used, reading every live script of the instance instead. Limited to the
-- workspaces the username exists in, so `g/all` shares elsewhere are not gathered; checked per
-- matched item through usr's primary key, as usr has no index on username alone.
CREATE FUNCTION shared_runnable_keys() RETURNS jsonb
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = public
AS $$
    WITH me AS (
        SELECT current_setting('session.user') AS username,
               array_append(regexp_split_to_array(current_setting('session.pgroups'), ','),
                            concat('u/', current_setting('session.user'))) AS keys
    )
    SELECT coalesce(jsonb_object_agg(shared.key, true), '{}'::jsonb)
    FROM (
        SELECT 'script:' || script.workspace_id || ':' || script.path AS key
        FROM script
        WHERE script.extra_perms ?| (SELECT keys FROM me)
          AND NOT script.archived AND NOT script.deleted
          AND EXISTS (SELECT 1 FROM usr WHERE usr.workspace_id = script.workspace_id AND usr.username = (SELECT username FROM me))
        UNION
        SELECT 'flow:' || flow.workspace_id || ':' || flow.path
        FROM flow
        WHERE flow.extra_perms ?| (SELECT keys FROM me)
          AND NOT flow.archived
          AND EXISTS (SELECT 1 FROM usr WHERE usr.workspace_id = flow.workspace_id AND usr.username = (SELECT username FROM me))
    ) shared
$$;

-- The keys are gathered once per statement and each row is a key lookup. Do not turn this into
-- `runnable_path = ANY (array)`: that compares every scanned row against the whole list, and a
-- user whose shared items rarely run makes the runs list scan the whole table (22 s at 1,000
-- shared items on 3M jobs, against 1.4 s with the lookup).
-- Only runs of the script or flow itself match: other kinds get no key, among them
-- `singlestepflow`, whose runnable_path may name either a script or a flow.
CREATE POLICY see_item_extra_perms ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING (
        visible_to_owner IS TRUE
        AND (SELECT shared_runnable_keys()) ?
            (CASE kind WHEN 'script' THEN 'script:' WHEN 'flow' THEN 'flow:' END
             || workspace_id || ':' || runnable_path)
    );
