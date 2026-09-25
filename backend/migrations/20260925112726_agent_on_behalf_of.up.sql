-- The identity an AI agent runs as when someone runs it from its own page, as a flow's or a
-- script's `on_behalf_of` is: a principal (`u/<name>`, `g/<group>`), resolved at deploy time.
-- Only `ai_agent` resources read it; every other resource type leaves it NULL.
ALTER TABLE resource ADD COLUMN IF NOT EXISTS on_behalf_of VARCHAR(255);

-- An agent's value is code it runs (inline tools, instructions), so a write of it that does not
-- say who the agent now runs as must not keep running it as whoever wrote the previous one:
-- anyone who can write the value would otherwise act with that account's permissions. The API's
-- writers set the column again in the same transaction; any other path (a direct SQL update, a
-- writer added later) leaves it cleared, which runs the agent as whoever runs it.
CREATE OR REPLACE FUNCTION reset_agent_on_behalf_of() RETURNS trigger AS $$
BEGIN
    IF NEW.resource_type = 'ai_agent'
       AND NEW.value IS DISTINCT FROM OLD.value
       AND NEW.on_behalf_of IS NOT DISTINCT FROM OLD.on_behalf_of THEN
        NEW.on_behalf_of := NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER reset_agent_on_behalf_of
    BEFORE UPDATE ON resource
    FOR EACH ROW EXECUTE FUNCTION reset_agent_on_behalf_of();

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

-- An agent saved before this column ran as whoever ran it; give it the account that wrote its
-- current version, the one a deploy now records. A principal wider than
-- `v2_job.permissioned_as` could not be enqueued, so it is left unset, as the sibling backfills
-- leave it (20260801043001, 20260911085221).
UPDATE resource r
   SET on_behalf_of = pg_temp.username_to_permissioned_as(v.created_by)
  FROM (
    SELECT DISTINCT ON (workspace_id, path) workspace_id, path, created_by
      FROM resource_version
     ORDER BY workspace_id, path, id DESC
  ) v
 WHERE r.workspace_id = v.workspace_id
   AND r.path = v.path
   AND r.resource_type = 'ai_agent'
   AND r.on_behalf_of IS NULL
   AND v.created_by IS NOT NULL
   AND v.created_by <> ''
   AND length(pg_temp.username_to_permissioned_as(v.created_by)) <= 55;

DROP FUNCTION pg_temp.username_to_permissioned_as(VARCHAR);
