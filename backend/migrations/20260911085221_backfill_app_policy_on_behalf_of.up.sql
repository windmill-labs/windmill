-- Add up migration script here
-- `policy.on_behalf_of` becomes the authority for an app's identity: the address beside it is
-- written through from it on every save, so the two can no longer name different accounts.
--
-- The address key is deliberately NOT removed here, and is still written: a replica predating
-- the derive-when-absent fallback errors outright when it is missing, which would 400 every
-- anonymous, publisher and guest app served by one that has not yet rolled over. Removing the
-- key is a follow-up, per docs/app-policy-email-removal.md.
--
-- What is left is the data written before that rule. A policy that only ever had the address has
-- no principal to run as, so give it one. A policy whose halves disagree was stored as a client
-- sent it; reads return that pair and a redeploy that keeps the identity sends it back, where the
-- pair check rejects it. So once every policy has a principal, rewrite its address from it.

-- Mirrors `users::username_to_permissioned_as`: an email-shaped username is its own principal
-- unless it contains a slash, which a reader would split on, and a legacy `group-*` username is
-- the group it names.
CREATE OR REPLACE FUNCTION pg_temp.username_to_permissioned_as(name VARCHAR)
RETURNS VARCHAR AS $$
    SELECT CASE
        WHEN $1 LIKE '%@%' AND $1 LIKE '%/%' THEN 'u/' || $1
        WHEN $1 LIKE '%@%' THEN $1
        WHEN $1 LIKE 'group-%' THEN 'g/' || substr($1, 7)
        ELSE 'u/' || $1
    END;
$$ LANGUAGE SQL IMMUTABLE;

-- Mirrors `users::permissioned_as_from_email`: a real account wins over the synthetic group
-- namespace, which is not reserved and may be a user's own address. `pg_temp` lives for the
-- whole session and migrations share one connection, so an identically-named helper from an
-- earlier migration is still in scope: replace it, and drop this one at the end.
CREATE OR REPLACE FUNCTION pg_temp.permissioned_as_from_email(w_id VARCHAR, email VARCHAR)
RETURNS VARCHAR AS $$
    SELECT COALESCE(
        (SELECT pg_temp.username_to_permissioned_as(u.username)
           FROM usr u WHERE u.workspace_id = $1 AND u.email = $2),
        -- A superadmin acting outside their workspaces has no usr row.
        (SELECT pg_temp.username_to_permissioned_as(COALESCE(p.username, p.email))
           FROM password p WHERE p.email = $2 AND p.super_admin),
        (SELECT 'g/' || g.name FROM group_ g
          WHERE g.workspace_id = $1
            AND $2 = 'group-' || g.name || '@windmill.dev')
    );
$$ LANGUAGE SQL STABLE;

-- Mirrors `users::get_email_from_permissioned_as`, except that a `u/` principal naming nobody
-- yields NULL rather than the synthetic `@unknown.windmill.dev` address, so that row is left as
-- it is instead of losing the one address it had.
CREATE OR REPLACE FUNCTION pg_temp.email_from_permissioned_as(w_id VARCHAR, principal VARCHAR)
RETURNS VARCHAR AS $$
    SELECT CASE
        WHEN $2 LIKE 'u/%' THEN COALESCE(
            (SELECT u.email FROM usr u WHERE u.workspace_id = $1 AND u.username = substr($2, 3)),
            (SELECT p.email FROM password p
              WHERE (p.username = substr($2, 3) OR p.email = substr($2, 3)) AND p.super_admin
              ORDER BY p.email LIMIT 1))
        WHEN $2 LIKE 'g/%' THEN 'group-' || substr($2, 3) || '@windmill.dev'
        ELSE $2
    END;
$$ LANGUAGE SQL STABLE;

-- A policy naming only the address predates the principal being written to it.
-- A principal wider than `v2_job.permissioned_as` could not be enqueued, so it is not recorded
-- at all — the app falls back to erroring on anonymous execution until someone picks an identity
-- the deploy path accepts. Same cap and reason as the sibling migration 20260801043001.
UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, policy->>'on_behalf_of_email')))
 WHERE policy->>'on_behalf_of' IS NULL
   AND pg_temp.permissioned_as_from_email(workspace_id, policy->>'on_behalf_of_email') IS NOT NULL
   AND length(pg_temp.permissioned_as_from_email(workspace_id, policy->>'on_behalf_of_email')) <= 55;

-- App drafts carry a copy of the policy and are deployed from it, so they need the same.
UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['policy', 'on_behalf_of'],
        to_jsonb(pg_temp.permissioned_as_from_email(workspace_id, value->'policy'->>'on_behalf_of_email'))))
 WHERE typ IN ('app', 'raw_app')
   AND value->'policy'->>'on_behalf_of' IS NULL
   AND pg_temp.permissioned_as_from_email(workspace_id, value->'policy'->>'on_behalf_of_email') IS NOT NULL
   AND length(pg_temp.permissioned_as_from_email(workspace_id, value->'policy'->>'on_behalf_of_email')) <= 55;

-- The address a save now writes, applied to the rows saved before. Execution already takes a `u/`
-- principal's own address, so this only changes what runs for a `g/` or bare principal, whose
-- stored address decided the superadmin flag and instance groups: those now follow the principal.
UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of_email'],
        to_jsonb(pg_temp.email_from_permissioned_as(workspace_id, policy->>'on_behalf_of')))
 WHERE policy->>'on_behalf_of' IS NOT NULL
   AND pg_temp.email_from_permissioned_as(workspace_id, policy->>'on_behalf_of') IS NOT NULL
   AND policy->>'on_behalf_of_email'
       IS DISTINCT FROM pg_temp.email_from_permissioned_as(workspace_id, policy->>'on_behalf_of');

UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['policy', 'on_behalf_of_email'],
        to_jsonb(pg_temp.email_from_permissioned_as(workspace_id, value->'policy'->>'on_behalf_of'))))
 WHERE typ IN ('app', 'raw_app')
   AND value->'policy'->>'on_behalf_of' IS NOT NULL
   AND pg_temp.email_from_permissioned_as(workspace_id, value->'policy'->>'on_behalf_of') IS NOT NULL
   AND value->'policy'->>'on_behalf_of_email'
       IS DISTINCT FROM pg_temp.email_from_permissioned_as(workspace_id, value->'policy'->>'on_behalf_of');

DROP FUNCTION pg_temp.permissioned_as_from_email(VARCHAR, VARCHAR);
DROP FUNCTION pg_temp.email_from_permissioned_as(VARCHAR, VARCHAR);
DROP FUNCTION pg_temp.username_to_permissioned_as(VARCHAR);
