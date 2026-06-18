-- Index supporting the recursive CTE in list_ws_specific_versions: each
-- iteration probes `WHERE ws.deploy_to = r.ws_id`, which without an index
-- on workspace_settings.deploy_to seq-scans the whole table per iteration
-- (up to depth-cap × |workspace_settings| row reads per call). deploy_to
-- is sparse — most workspaces don't deploy anywhere — so a partial index
-- keeps the index small while still covering every probe.
CREATE INDEX IF NOT EXISTS workspace_settings_deploy_to_idx
    ON workspace_settings (deploy_to)
    WHERE deploy_to IS NOT NULL;

-- Returns the list of workspace ids related to `seed_workspace` via the
-- `workspace_settings.deploy_to` graph (in either direction) for which a
-- `resource` or `variable` row at `item_path` exists AND is visible to
-- `user_email` under that workspace's RLS context.
--
-- For each related workspace this function:
--   1. resolves the user's per-workspace identity (`usr` row by email,
--      groups via `usr_to_group` + `email_to_igroup`, folders via
--      `folder.extra_perms`),
--   2. switches the role + session.* settings via `set_session_context`
--      so the next query is evaluated under that workspace's RLS,
--   3. runs an `EXISTS` against the requested item table.
--
-- This consolidates the "fan out per-workspace, set RLS, EXISTS" loop
-- that previously lived in the Rust handler into a single round trip.
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
                         WHEN ws.workspace_id = r.ws_id THEN ws.deploy_to
                         ELSE ws.workspace_id
                       END, r.depth + 1
                FROM workspace_settings ws, related_workspaces r
                WHERE r.depth < 32
                  AND ((ws.workspace_id = r.ws_id AND ws.deploy_to IS NOT NULL)
                       OR ws.deploy_to = r.ws_id)
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
