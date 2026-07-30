-- `workspace_settings.deploy_to` (2023) and `workspace.parent_workspace_id` (2025) both expressed
-- "which workspace does this one deploy into". Fork creation and dev-workspace attach seeded both,
-- but nothing kept them in agreement, so every reader had to pick one and they disagreed. Fold the
-- surviving `deploy_to` pairs into the lineage and drop the column.
--
-- A converted pair becomes a dev workspace when it can: that is what a long-lived staging paired
-- with prod actually is, and it keeps its own tag domain and promotion mode, so no worker
-- configuration changes underneath it. Only one dev workspace is allowed per parent
-- (`workspace_canonical_dev_idx`), so a pair becomes a plain fork instead when several workspaces
-- name the same target, or when that target already has a dev workspace. Those keep their link but
-- borrow the parent's job tags; an admin can re-attach one of them as the dev workspace afterwards.

DO $$
DECLARE
    skipped RECORD;
    converted_count INT;
BEGIN
    -- `parent_workspace_id` has an FK and every chain walker assumes a DAG rooted at a parentless
    -- node, so a pair that cannot be expressed that way is reported and left unlinked rather than
    -- forced into a shape that would break traversal.
    FOR skipped IN
        SELECT ws.workspace_id, ws.deploy_to,
               CASE
                   WHEN ws.deploy_to = ws.workspace_id THEN 'self-reference'
                   WHEN tgt.id IS NULL THEN 'target does not exist'
                   WHEN src.parent_workspace_id IS NOT NULL THEN 'already a fork or dev workspace'
                   WHEN tgt.parent_workspace_id IS NOT NULL THEN 'target is itself a fork'
                   ELSE 'chain: target also has a deploy target'
               END AS reason
        FROM workspace_settings ws
        JOIN workspace src ON src.id = ws.workspace_id
        LEFT JOIN workspace tgt ON tgt.id = ws.deploy_to
        LEFT JOIN workspace_settings tgt_ws ON tgt_ws.workspace_id = ws.deploy_to
        WHERE ws.deploy_to IS NOT NULL
          AND (ws.deploy_to = ws.workspace_id
               OR tgt.id IS NULL
               OR src.parent_workspace_id IS NOT NULL
               OR tgt.parent_workspace_id IS NOT NULL
               OR tgt_ws.deploy_to IS NOT NULL)
    LOOP
        RAISE NOTICE 'deploy_to unification: skipping % -> % (%)',
            skipped.workspace_id, skipped.deploy_to, skipped.reason;
    END LOOP;

    CREATE TEMP TABLE convertible ON COMMIT DROP AS
        SELECT ws.workspace_id, ws.deploy_to,
               -- Sole claimant on a target that has no dev workspace yet: the pairing survives
               -- as-is. Otherwise the one-dev-per-parent index forces a plain fork.
               (COUNT(*) OVER (PARTITION BY ws.deploy_to) = 1
                AND NOT EXISTS (
                    SELECT 1 FROM workspace d
                    WHERE d.parent_workspace_id = ws.deploy_to
                      AND d.is_dev_workspace AND NOT d.deleted
                )) AS as_dev
        FROM workspace_settings ws
        JOIN workspace src ON src.id = ws.workspace_id
        JOIN workspace tgt ON tgt.id = ws.deploy_to
        LEFT JOIN workspace_settings tgt_ws ON tgt_ws.workspace_id = ws.deploy_to
        WHERE ws.deploy_to IS NOT NULL
          AND ws.deploy_to <> ws.workspace_id
          AND src.parent_workspace_id IS NULL
          AND tgt.parent_workspace_id IS NULL
          AND tgt_ws.deploy_to IS NULL;

    FOR skipped IN SELECT workspace_id, deploy_to FROM convertible WHERE NOT as_dev
    LOOP
        RAISE NOTICE 'deploy_to unification: % -> % becomes a plain fork (target already has a dev workspace, or several workspaces name it); its jobs will use the parent''s tags',
            skipped.workspace_id, skipped.deploy_to;
    END LOOP;

    UPDATE workspace w
       SET parent_workspace_id = c.deploy_to,
           is_dev_workspace = c.as_dev
      FROM convertible c
     WHERE w.id = c.workspace_id;

    GET DIAGNOSTICS converted_count = ROW_COUNT;
    RAISE NOTICE 'deploy_to unification: linked % workspace(s) to their parent (% as dev workspaces)',
        converted_count, (SELECT count(*) FROM convertible WHERE as_dev);
END $$;

-- Resolve workspace-specific resources/variables over the fork lineage instead of the `deploy_to`
-- graph. Only the traversal changes; the per-workspace RLS fan-out below is unchanged. This also
-- closes a gap where forks were never considered related at all.
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
            WITH RECURSIVE related_workspaces(ws_id, depth) AS (
                SELECT seed_workspace::VARCHAR, 0
              UNION
                SELECT CASE
                         WHEN w.id = r.ws_id THEN w.parent_workspace_id
                         ELSE w.id
                       END, r.depth + 1
                FROM workspace w, related_workspaces r
                WHERE r.depth < 32
                  AND ((w.id = r.ws_id AND w.parent_workspace_id IS NOT NULL)
                       OR w.parent_workspace_id = r.ws_id)
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
