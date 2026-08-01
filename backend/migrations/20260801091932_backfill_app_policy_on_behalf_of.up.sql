-- Add up migration script here
-- `policy.on_behalf_of` becomes the authority for an app's identity: the address beside it is
-- derived from it on every read and written through on every save, so the two can no longer
-- name different accounts.
--
-- The address key is deliberately NOT removed here. The previous version's `get_on_behalf_of`
-- errors outright when it is absent, so stripping it would 400 every anonymous and publisher
-- app served by an API replica that has not yet rolled over. Removing the key is a follow-up,
-- once no supported version reads it.
--
-- What is left is the backfill: a policy that only ever had the address has no principal for
-- the read path to derive from, so give it one.

-- Mirrors `users::permissioned_as_from_email`: a real account wins over the synthetic group
-- namespace, which is not reserved and may be a user's own address. `pg_temp` lives for the
-- whole session and migrations share one connection, so an identically-named helper from an
-- earlier migration is still in scope: replace it, and drop this one at the end.
CREATE OR REPLACE FUNCTION pg_temp.permissioned_as_from_email(w_id VARCHAR, email VARCHAR)
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

-- A policy naming only the address predates the principal being written to it.
UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, policy->>'on_behalf_of_email')))
 WHERE policy->>'on_behalf_of' IS NULL
   AND pg_temp.permissioned_as_from_email(workspace_id, policy->>'on_behalf_of_email') IS NOT NULL;

-- App drafts carry a copy of the policy and are deployed from it, so they need the same.
UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['policy', 'on_behalf_of'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, value->'policy'->>'on_behalf_of_email'))))
 WHERE typ IN ('app', 'raw_app')
   AND value->'policy'->>'on_behalf_of' IS NULL
   AND pg_temp.permissioned_as_from_email(workspace_id, value->'policy'->>'on_behalf_of_email') IS NOT NULL;

DROP FUNCTION pg_temp.permissioned_as_from_email(VARCHAR, VARCHAR);
