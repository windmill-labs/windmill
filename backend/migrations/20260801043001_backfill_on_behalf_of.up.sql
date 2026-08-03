-- Add up migration script here
-- The permissioned_as becomes the identity this release reads; the email is derived from it at
-- read time, as triggers have always done. Backfilling it also retires the fallback to
-- created_by / edited_by, so a runnable deployed before the column existed starts running
-- as the user its on_behalf_of_email always named rather than as whoever last deployed it.
--
-- `on_behalf_of_email` stays for now: a worker predating this release resolves a script or flow
-- through `get_script_info_for_hash` / `get_latest_hash_for_path`, which select that column, and
-- workers are expected to lag the server. Deploys keep writing it until every live worker is new
-- (MIN_VERSION_SUPPORTS_ON_BEHALF_OF_PRINCIPAL); a later release stops writing it and drops it.
--
-- That later migration MUST re-run this backfill before dropping: server pods are mixed for the
-- minute or two a rollout takes, and one still on the previous release writes only the address —
-- leaving a runnable deployed in that window with no principal, which reads as no identity at
-- all and runs it as its caller. Re-deriving picks those up; dropping without it makes them
-- permanent.

-- Mirrors `users::permissioned_as_from_email`: a real account wins over the synthetic group
-- namespace, which is not reserved and may be a user's own address.
CREATE FUNCTION pg_temp.permissioned_as_from_email(w_id VARCHAR, email VARCHAR)
RETURNS VARCHAR AS $$
    SELECT COALESCE(
        -- `username_to_permissioned_as`: an email-shaped username is its own principal unless
        -- it contains a slash, which a reader would split on.
        (SELECT CASE WHEN u.username LIKE '%@%' AND u.username NOT LIKE '%/%' THEN u.username
                     ELSE 'u/' || u.username END
           FROM usr u WHERE u.workspace_id = $1 AND u.email = $2),
        -- A superadmin acting outside their workspaces has no usr row.
        (SELECT CASE WHEN COALESCE(p.username, p.email) LIKE '%@%'
                      AND COALESCE(p.username, p.email) NOT LIKE '%/%'
                     THEN COALESCE(p.username, p.email)
                     ELSE 'u/' || COALESCE(p.username, p.email) END
           FROM password p WHERE p.email = $2 AND p.super_admin),
        (SELECT 'g/' || g.name FROM group_ g
          WHERE g.workspace_id = $1
            AND $2 = 'group-' || g.name || '@windmill.dev')
    );
$$ LANGUAGE SQL STABLE;

-- A principal wider than `v2_job.permissioned_as` could not be enqueued, so it is not recorded
-- at all — the runnable falls back to running as its caller until someone picks an identity the
-- deploy path accepts. Only an address-derived principal can reach that width; a username is
-- capped at 50.
UPDATE script SET on_behalf_of =
    pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)
 WHERE on_behalf_of_email IS NOT NULL AND on_behalf_of IS NULL
   AND length(pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)) <= 55;

UPDATE flow SET on_behalf_of =
    pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)
 WHERE on_behalf_of_email IS NOT NULL AND on_behalf_of IS NULL
   AND length(pg_temp.permissioned_as_from_email(workspace_id, on_behalf_of_email)) <= 55;

-- Drafts carry the same pair in their value.
UPDATE draft SET value = to_json(jsonb_set(
        to_jsonb(value),
        ARRAY['on_behalf_of'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, value->>'on_behalf_of_email'))))
 WHERE typ IN ('script', 'flow')
   AND value->>'on_behalf_of_email' IS NOT NULL
   AND value->>'on_behalf_of' IS NULL
   AND length(pg_temp.permissioned_as_from_email(workspace_id, value->>'on_behalf_of_email')) <= 55;

