-- `workspace_settings.deploy_to` (2023) and `workspace.parent_workspace_id` (2025) both expressed
-- "which workspace does this one deploy into". Fork creation and dev-workspace attach seeded both,
-- but nothing kept them in agreement, so every reader had to pick one and they disagreed. Fold the
-- surviving `deploy_to` pairs into the lineage and drop the column.
--
-- A converted pair becomes a dev workspace when it can: that is what a long-lived staging paired
-- with prod actually is, and it keeps its own tag domain and promotion mode, so no worker
-- configuration changes underneath it. Only one dev workspace is allowed per parent
-- (`workspace_canonical_dev_idx`) and the app only ever attaches a dev to a root, so a pair becomes
-- a plain fork when it fails either test. Those keep their link but borrow the parent's job tags;
-- an admin can re-attach one of them as the dev workspace afterwards.
--
-- Chains (`dev -> staging -> prod`) convert too: `parent_workspace_id` represents them natively, up
-- to the depth-20 backstop that billing and count resolution walk.

-- Whatever the lineage genuinely cannot express is preserved here rather than destroyed with the
-- column. `deploy_to` was admin-settable to any workspace, so these rows are real configuration; an
-- operator needs to see what was dropped, and the down migration restores from this table.
CREATE TABLE workspace_deploy_to_unmigrated (
    workspace_id VARCHAR(50) PRIMARY KEY REFERENCES workspace(id) ON DELETE CASCADE,
    deploy_to VARCHAR(255) NOT NULL,
    reason TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE workspace_deploy_to_unmigrated IS
    'Legacy workspace_settings.deploy_to links that could not be expressed as fork lineage when the column was dropped. Written once by migration 20260730080304; never written by the application. Dropped again by that migration when every link converted, so it only exists where something was preserved.';

-- Tables created after the one-time GRANT ALL in 20250205131523 need explicit grants: ALTER
-- DEFAULT PRIVILEGES only covers objects created by the role that set them. Without these the
-- operator this table exists for cannot read it through the application's role.
GRANT ALL ON workspace_deploy_to_unmigrated TO windmill_user;
GRANT ALL ON workspace_deploy_to_unmigrated TO windmill_admin;

DO $$
DECLARE
    leftover RECORD;
    demoted RECORD;
    converted_count INT;
    dev_count INT;
BEGIN
    -- Rows that could become lineage at all: a live source with no parent yet, pointing at a live
    -- workspace other than itself.
    CREATE TEMP TABLE eligible ON COMMIT DROP AS
        SELECT ws.workspace_id, ws.deploy_to
        FROM workspace_settings ws
        JOIN workspace src ON src.id = ws.workspace_id
        JOIN workspace tgt ON tgt.id = ws.deploy_to
        WHERE ws.deploy_to IS NOT NULL
          AND ws.deploy_to <> ws.workspace_id
          AND NOT src.deleted AND NOT tgt.deleted
          AND src.parent_workspace_id IS NULL
          AND NOT EXISTS (
              SELECT 1 FROM workspace d
              WHERE d.parent_workspace_id = ws.workspace_id
                AND d.is_dev_workspace AND NOT d.deleted
          );

    -- Cycle detection has to run over the lineage as it would exist AFTER conversion, not over the
    -- `deploy_to` graph alone: a root whose target is one of its own existing forks would close a
    -- loop that no `deploy_to` edge reveals. Every eligible source is parentless, so the combined
    -- edge set still gives each node at most one parent.
    CREATE TEMP TABLE edge ON COMMIT DROP AS
        SELECT id AS child, parent_workspace_id AS parent
        FROM workspace WHERE parent_workspace_id IS NOT NULL
      UNION ALL
        SELECT workspace_id, deploy_to FROM eligible;

    CREATE TEMP TABLE chain_info ON COMMIT DROP AS
        WITH RECURSIVE walk(start_id, cur_id, depth, path, cyclic) AS (
            SELECT e.child, e.parent, 1, ARRAY[e.child, e.parent], e.parent = e.child
            FROM edge e
          UNION ALL
            SELECT w.start_id, nxt.parent, w.depth + 1,
                   w.path || nxt.parent, nxt.parent = ANY(w.path)
            FROM walk w
            JOIN edge nxt ON nxt.child = w.cur_id
            WHERE NOT w.cyclic AND w.depth < 25
        )
        SELECT start_id, bool_or(cyclic) AS cyclic, max(depth) AS chain_depth
        FROM walk GROUP BY start_id;

    -- Every remaining link gets a verdict. A fork whose `deploy_to` already names its parent is the
    -- redundant seed every fork carried, so dropping it loses nothing and it is not reported.
    CREATE TEMP TABLE classified ON COMMIT DROP AS
        SELECT ws.workspace_id, ws.deploy_to,
               CASE
                   WHEN src.parent_workspace_id = ws.deploy_to THEN 'redundant'
                   WHEN src.deleted THEN 'source workspace is archived'
                   WHEN ws.deploy_to = ws.workspace_id THEN 'self-reference'
                   WHEN tgt.id IS NULL THEN 'target workspace does not exist'
                   WHEN tgt.deleted THEN 'target workspace is archived'
                   WHEN src.parent_workspace_id IS NOT NULL
                       THEN 'already a fork, pointing somewhere other than its parent'
                   WHEN EXISTS (
                       SELECT 1 FROM workspace d
                       WHERE d.parent_workspace_id = ws.workspace_id
                         AND d.is_dev_workspace AND NOT d.deleted
                   ) THEN 'source owns a dev workspace; linking it would nest that dev under a fork'
                   WHEN ci.cyclic THEN 'linking it would form a cycle in the workspace lineage'
                   WHEN ci.chain_depth > 20 THEN 'chain exceeds the lineage depth limit'
               END AS reason
        FROM workspace_settings ws
        JOIN workspace src ON src.id = ws.workspace_id
        LEFT JOIN workspace tgt ON tgt.id = ws.deploy_to
        LEFT JOIN chain_info ci ON ci.start_id = ws.workspace_id
        WHERE ws.deploy_to IS NOT NULL;

    INSERT INTO workspace_deploy_to_unmigrated (workspace_id, deploy_to, reason)
        SELECT workspace_id, deploy_to, reason
        FROM classified WHERE reason IS NOT NULL AND reason <> 'redundant';

    FOR leftover IN SELECT workspace_id, deploy_to, reason FROM workspace_deploy_to_unmigrated
    LOOP
        RAISE WARNING 'deploy_to unification: % -> % kept in workspace_deploy_to_unmigrated (%)',
            leftover.workspace_id, leftover.deploy_to, leftover.reason;
    END LOOP;

    CREATE TEMP TABLE convertible ON COMMIT DROP AS
        SELECT c.workspace_id, c.deploy_to,
               -- Sole claimant on a free root that has no dev workspace yet, mirroring what
               -- `attach_dev_workspace` would allow. Anything else stays a plain fork.
               (COUNT(*) OVER (PARTITION BY c.deploy_to) = 1
                AND tgt.parent_workspace_id IS NULL
                AND NOT EXISTS (
                    SELECT 1 FROM workspace d
                    WHERE d.parent_workspace_id = c.deploy_to
                      AND d.is_dev_workspace AND NOT d.deleted
                )
                AND NOT EXISTS (
                    SELECT 1 FROM classified c2
                    WHERE c2.workspace_id = c.deploy_to AND c2.reason IS NULL
                )) AS as_dev
        FROM classified c
        JOIN workspace tgt ON tgt.id = c.deploy_to
        WHERE c.reason IS NULL;

    FOR demoted IN SELECT workspace_id, deploy_to FROM convertible WHERE NOT as_dev
    LOOP
        RAISE NOTICE 'deploy_to unification: % -> % becomes a plain fork (target is not a free root); its jobs will use the parent''s tags',
            demoted.workspace_id, demoted.deploy_to;
    END LOOP;

    UPDATE workspace w
       SET parent_workspace_id = c.deploy_to,
           is_dev_workspace = c.as_dev
      FROM convertible c
     WHERE w.id = c.workspace_id;
    GET DIAGNOSTICS converted_count = ROW_COUNT;

    -- Dispatch ignores this flag once a workspace has a parent, but the stored `true` outlives the
    -- pairing: detaching later would silently re-enable instance alerting nobody asked for.
    -- `attach_dev_workspace` clears it for the same reason.
    UPDATE workspace_settings ws
       SET error_handler_fallback_to_instance_alerts = false
      FROM convertible c
     WHERE c.workspace_id = ws.workspace_id
       AND ws.error_handler_fallback_to_instance_alerts;

    -- A converted workspace is now parent-managed, exactly as if `attach_dev_workspace` had run.
    -- That path also strips git-sync state that would otherwise keep pulling and pushing against
    -- the workspace's pre-conversion tracked branch: promotion repos are dropped, and auto-pull and
    -- fork PRs are cleared on the rest. Any managed webhook is left registered but inert -- the
    -- migration cannot call GitHub -- and is cleaned up on the next settings save.
    UPDATE workspace_settings ws
       SET git_sync = jsonb_set(
               ws.git_sync,
               '{repositories}',
               COALESCE((
                   SELECT jsonb_agg(
                              (elem - 'auto_pull' - 'open_pr_error')
                              || jsonb_build_object('fork_open_prs', false))
                   FROM jsonb_array_elements(ws.git_sync->'repositories') AS elem
                   WHERE COALESCE((elem->>'use_individual_branch')::boolean, false) = false
               ), '[]'::jsonb)
           )
      FROM convertible c
     WHERE c.workspace_id = ws.workspace_id
       AND jsonb_typeof(ws.git_sync->'repositories') = 'array';

    SELECT count(*) INTO dev_count FROM convertible WHERE as_dev;
    RAISE NOTICE 'deploy_to unification: linked % workspace(s) to their parent (% as dev workspaces), % preserved in workspace_deploy_to_unmigrated',
        converted_count, dev_count, (SELECT count(*) FROM workspace_deploy_to_unmigrated);

    -- Nothing to preserve is the normal outcome; leaving an empty table behind on every instance
    -- forever buys nothing. It survives only where it holds something an operator needs to see.
    IF NOT EXISTS (SELECT 1 FROM workspace_deploy_to_unmigrated) THEN
        DROP TABLE workspace_deploy_to_unmigrated;
    END IF;
END $$;

-- Resolve workspace-specific resources/variables over the fork lineage instead of the `deploy_to`
-- graph. Only the traversal changes; the per-workspace RLS fan-out below is unchanged.
--
-- Walks up through ancestors and down only to a dev workspace: descending into plain forks would
-- fan a root out over its whole live fork subtree, and each member costs an RLS switch and probe.
CREATE OR REPLACE FUNCTION list_ws_specific_versions(
    seed_workspace TEXT,
    user_email TEXT,
    item_kind TEXT,
    item_path TEXT
) RETURNS TABLE(ws VARCHAR) AS $$
DECLARE
    rel RECORD;
    usr_row RECORD;
    user_perms TEXT[];
    groups_csv TEXT;
    pgroups_csv TEXT;
    folders_read_csv TEXT;
    folders_write_csv TEXT;
    item_exists BOOLEAN;
    is_super BOOLEAN;
BEGIN
    IF item_kind NOT IN ('resource', 'variable') THEN
        RAISE EXCEPTION 'Invalid kind: %', item_kind;
    END IF;

    SELECT COALESCE(super_admin, false) INTO is_super
    FROM password WHERE email = user_email;
    is_super := COALESCE(is_super, false);

    BEGIN
        FOR rel IN
            WITH RECURSIVE related_workspaces(ws_id, depth, seen) AS (
                SELECT seed_workspace::VARCHAR, 0, ARRAY[seed_workspace::VARCHAR]
              UNION ALL
                SELECT step.next_id, r.depth + 1, r.seen || step.next_id
                FROM related_workspaces r
                CROSS JOIN LATERAL (
                    SELECT CASE WHEN w.id = r.ws_id THEN w.parent_workspace_id ELSE w.id END
                               AS next_id
                    FROM workspace w
                    WHERE (w.id = r.ws_id AND w.parent_workspace_id IS NOT NULL)
                       OR (w.parent_workspace_id = r.ws_id
                           AND w.is_dev_workspace AND NOT w.deleted)
                ) step
                -- The edges run both ways, so without this the walk bounces parent<->dev until it
                -- hits the depth cap on every call regardless of how few workspaces are related.
                WHERE r.depth < 32 AND NOT (step.next_id = ANY(r.seen))
            )
            SELECT DISTINCT r.ws_id
            FROM related_workspaces r
            INNER JOIN workspace w ON w.id = r.ws_id AND w.deleted = false
        LOOP
            SELECT u.username, u.is_admin
            INTO usr_row
            FROM usr u
            WHERE u.email = user_email
              AND u.workspace_id = rel.ws_id
              AND u.disabled = false;

            IF NOT FOUND AND NOT is_super THEN
                CONTINUE;
            END IF;

            IF NOT FOUND THEN
                -- super admin without a usr row in this workspace: synthesize an
                -- admin identity so RLS is bypassed (windmill_admin role).
                usr_row.username := user_email;
                usr_row.is_admin := true;
                groups_csv := '';
                pgroups_csv := '';
                folders_read_csv := '';
                folders_write_csv := '';
            ELSE
                SELECT
                    COALESCE(string_agg(g, ','), ''),
                    COALESCE(string_agg('g/' || g, ','), '')
                INTO groups_csv, pgroups_csv
                FROM (
                    SELECT group_ AS g FROM usr_to_group
                    WHERE usr_to_group.usr = usr_row.username
                      AND usr_to_group.workspace_id = rel.ws_id
                  UNION ALL
                    SELECT igroup FROM email_to_igroup WHERE email = user_email
                ) gs;

                user_perms := ARRAY['u/' || usr_row.username] || ARRAY(
                    SELECT 'g/' || g FROM (
                        SELECT group_ AS g FROM usr_to_group
                        WHERE usr = usr_row.username AND workspace_id = rel.ws_id
                      UNION ALL
                        SELECT igroup FROM email_to_igroup WHERE email = user_email
                    ) gs2
                );

                -- folders_read: every folder the user can see (write implies read);
                -- folders_write: only those granting write access.
                WITH user_folders AS (
                    SELECT name, EXISTS (
                        SELECT 1 FROM jsonb_each_text(extra_perms) t
                        WHERE t.key = ANY(user_perms) AND t.value::boolean IS true
                    ) AS is_write
                    FROM folder
                    WHERE extra_perms ?| user_perms AND folder.workspace_id = rel.ws_id
                )
                SELECT
                    COALESCE(string_agg(name, ','), ''),
                    COALESCE(string_agg(name, ',') FILTER (WHERE is_write), '')
                INTO folders_read_csv, folders_write_csv
                FROM user_folders;

                IF is_super THEN
                    usr_row.is_admin := true;
                END IF;
            END IF;

            PERFORM set_session_context(
                usr_row.is_admin,
                usr_row.username,
                groups_csv,
                pgroups_csv,
                folders_read_csv,
                folders_write_csv
            );

            EXECUTE format(
                'SELECT EXISTS(SELECT 1 FROM %I WHERE workspace_id = $1 AND path = $2)',
                item_kind
            )
            INTO item_exists
            USING rel.ws_id, item_path;

            IF item_exists THEN
                ws := rel.ws_id;
                RETURN NEXT;
            END IF;
        END LOOP;
    EXCEPTION WHEN OTHERS THEN
        -- Reset to a deny-default state before re-raising so a half-set
        -- session context can't leak past the failed call.
        PERFORM set_session_context(false, '', '', '', '', '');
        RAISE;
    END;

    -- Reset to a deny-default state on the happy path too. SET LOCAL is
    -- transaction-scoped so this also unwinds at transaction end, but
    -- being explicit defends against the function being called inside a
    -- longer outer transaction.
    PERFORM set_session_context(false, '', '', '', '', '');
END;
$$ LANGUAGE plpgsql;

-- Dropping the column takes `workspace_settings_deploy_to_idx` with it; the new traversal is served
-- by `workspace_parent_idx`.
ALTER TABLE workspace_settings DROP COLUMN deploy_to;
