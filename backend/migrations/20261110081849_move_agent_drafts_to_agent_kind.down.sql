-- A `resource` row at the same (path, owner) would collide on the
-- draft_pkey_with_user / draft_pkey_legacy unique indexes; the agent row is the
-- one the agent editor wrote, so it wins.
DELETE FROM draft d
WHERE d.typ = 'resource'
  AND EXISTS (
    SELECT 1 FROM draft a
    WHERE a.workspace_id = d.workspace_id
      AND a.path = d.path
      AND a.typ = 'agent'
      AND a.email IS NOT DISTINCT FROM d.email
  );

UPDATE draft SET typ = 'resource' WHERE typ = 'agent';
