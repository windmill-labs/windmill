-- Retag every agent's draft (any owner, the legacy NULL-email row included) from
-- `resource` to `agent`. The deployed row decides when there is one, since a
-- deploy keeps the row's type whatever the draft names; a draft-only agent is
-- known by the `resource_type` its draft names. `->>` reads the `json` column
-- without a cast, which a stored NUL escape would make raise.
UPDATE draft d
SET typ = 'agent'
WHERE d.typ = 'resource'
  AND COALESCE(
    (SELECT r.resource_type = 'ai_agent'
     FROM resource r
     WHERE r.workspace_id = d.workspace_id AND r.path = d.path),
    d.value ->> 'resource_type' = 'ai_agent',
    false
  );
