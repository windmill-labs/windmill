-- Add up migration script here
-- The permissioned_as becomes the only stored identity; the email is derived from it at
-- read time, as triggers have always done. Backfilling it also retires the fallback to
-- created_by / edited_by, so a runnable deployed before the column existed starts running
-- as the user its on_behalf_of_email always named rather than as whoever last deployed it.

-- Mirrors `users::permissioned_as_from_email`: a real account wins over the synthetic group
-- namespace, which is not reserved and may be a user's own address.
CREATE FUNCTION pg_temp.permissioned_as_from_email(w_id VARCHAR, email VARCHAR)
RETURNS VARCHAR AS $$
    SELECT COALESCE(
        -- `username_to_permissioned_as`: an email-shaped username is stored verbatim.
        (SELECT CASE WHEN u.username LIKE '%@%' THEN u.username ELSE 'u/' || u.username END
           FROM usr u WHERE u.workspace_id = $1 AND u.email = $2),
        -- A superadmin acting outside their workspaces has no usr row.
        (SELECT CASE WHEN COALESCE(p.username, p.email) LIKE '%@%' THEN COALESCE(p.username, p.email)
                     ELSE 'u/' || p.username END
           FROM password p WHERE p.email = $2 AND p.super_admin),
        (SELECT 'g/' || g.name FROM group_ g
          WHERE g.workspace_id = $1
            AND $2 = 'group-' || g.name || '@windmill.dev')
    );
$$ LANGUAGE SQL STABLE;

UPDATE script SET on_behalf_of_permissioned_as =
    pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)
 WHERE on_behalf_of_email IS NOT NULL AND on_behalf_of_permissioned_as IS NULL;

UPDATE flow SET on_behalf_of_permissioned_as =
    pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)
 WHERE on_behalf_of_email IS NOT NULL AND on_behalf_of_permissioned_as IS NULL;

-- Drafts carry the same pair in their value.
UPDATE draft SET value = to_json(jsonb_set(
        to_jsonb(value),
        ARRAY['on_behalf_of_permissioned_as'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, value->>'on_behalf_of_email'))))
 WHERE typ IN ('script', 'flow')
   AND value->>'on_behalf_of_email' IS NOT NULL
   AND value->>'on_behalf_of_permissioned_as' IS NULL
   AND pg_temp.permissioned_as_from_email(workspace_id, value->>'on_behalf_of_email') IS NOT NULL;

ALTER TABLE script DROP COLUMN IF EXISTS on_behalf_of_email;
ALTER TABLE flow DROP COLUMN IF EXISTS on_behalf_of_email;
