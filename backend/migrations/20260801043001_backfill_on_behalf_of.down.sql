-- Add down migration script here
-- The column was never dropped, so this only re-derives it from the principal for the rows the
-- up migration backfilled. One naming a group gets that group's synthetic address, which is what
-- the column held before.

CREATE FUNCTION pg_temp.email_from_permissioned_as(w_id VARCHAR, permissioned_as VARCHAR)
RETURNS VARCHAR AS $$
    -- Prefix first, as every reader decides: a group name or an email-shaped username may
    -- itself contain '@', so only an unprefixed value is an address.
    SELECT CASE
        WHEN $2 LIKE 'g/%'
            THEN 'group-' || substring($2 from 3) || '@windmill.dev'
        -- Mirrors the up migration's `u/` arm, superadmin fallback included: one acting
        -- outside their workspaces has no usr row, and losing their address here would
        -- leave the previous runtime unable to authenticate the runnable.
        WHEN $2 LIKE 'u/%'
            THEN COALESCE(
                (SELECT u.email FROM usr u WHERE u.workspace_id = $1 AND u.username = substring($2 from 3)),
                (SELECT p.email FROM password p WHERE p.super_admin
                   AND (p.username = substring($2 from 3) OR p.email = substring($2 from 3))
                 ORDER BY p.email LIMIT 1),
                substring($2 from 3) || '@unknown.windmill.dev')
        ELSE $2
    END;
$$ LANGUAGE SQL STABLE;

UPDATE script SET on_behalf_of_email =
    pg_temp.email_from_permissioned_as(workspace_id, on_behalf_of)
 WHERE on_behalf_of IS NOT NULL;

UPDATE flow SET on_behalf_of_email =
    pg_temp.email_from_permissioned_as(workspace_id, on_behalf_of)
 WHERE on_behalf_of IS NOT NULL;
