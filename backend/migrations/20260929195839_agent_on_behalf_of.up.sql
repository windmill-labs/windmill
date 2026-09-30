-- An AI agent's value carries the identity it runs as when run from its own page, as a flow's or
-- a script's `on_behalf_of` is: a principal (`u/<name>`, `g/<group>`), resolved by the server on
-- every write of the agent. An agent saved before this ran as whoever ran it; give it the
-- account that wrote its current version, the one a write now records.

-- Mirrors `users::username_to_permissioned_as`, as the sibling backfills do. `pg_temp` lives for
-- the whole session and migrations share one connection, so replace any earlier copy and drop
-- this one at the end.
CREATE OR REPLACE FUNCTION pg_temp.username_to_permissioned_as(name VARCHAR)
RETURNS VARCHAR AS $$
    SELECT CASE
        WHEN $1 LIKE '%@%' AND $1 LIKE '%/%' THEN 'u/' || $1
        WHEN $1 LIKE '%@%' THEN $1
        WHEN $1 LIKE 'group-%' THEN 'g/' || substr($1, 7)
        ELSE 'u/' || $1
    END;
$$ LANGUAGE SQL IMMUTABLE;

-- Filling in the identity is not an edit of the agent, so it records no version.
ALTER TABLE resource DISABLE TRIGGER record_resource_version_update_trigger;

-- A principal wider than `v2_job.permissioned_as` could not be enqueued, so it is left unset, as
-- the sibling backfills leave it (20260801043001, 20260911085221).
UPDATE resource r
   SET value = jsonb_set(r.value, '{on_behalf_of}',
                         to_jsonb(pg_temp.username_to_permissioned_as(v.created_by)))
  FROM (
    SELECT DISTINCT ON (workspace_id, path) workspace_id, path, created_by
      FROM resource_version
     ORDER BY workspace_id, path, id DESC
  ) v
 WHERE r.workspace_id = v.workspace_id
   AND r.path = v.path
   AND r.resource_type = 'ai_agent'
   AND jsonb_typeof(r.value) = 'object'
   AND NOT (r.value ? 'on_behalf_of')
   AND v.created_by IS NOT NULL
   AND v.created_by <> ''
   AND length(pg_temp.username_to_permissioned_as(v.created_by)) <= 55;

ALTER TABLE resource ENABLE TRIGGER record_resource_version_update_trigger;

DROP FUNCTION pg_temp.username_to_permissioned_as(VARCHAR);
