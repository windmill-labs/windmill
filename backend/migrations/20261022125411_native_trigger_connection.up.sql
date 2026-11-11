-- Each native trigger names the connection (OAuth resource) it acts through, instead of every
-- trigger of a service sharing the single workspace connection.
ALTER TABLE native_trigger ADD COLUMN connection_path VARCHAR(255);

-- Saving the OAuth app, or starting a connect with the instance app, used to clear
-- resource_path while the connection itself stayed. A service then had at most one connection,
-- so when exactly one is found it is the one its triggers used.
UPDATE workspace_integrations wi
SET resource_path = (
    SELECT CASE WHEN count(*) = 1 THEN min(v.path) END
    FROM variable v
    JOIN account a ON a.workspace_id = v.workspace_id AND a.id = v.account
    WHERE v.workspace_id = wi.workspace_id
      AND a.client = wi.service_name::text
      AND a.is_workspace_integration = true
)
WHERE wi.resource_path IS NULL;

UPDATE native_trigger nt
SET connection_path = wi.resource_path
FROM workspace_integrations wi
WHERE wi.workspace_id = nt.workspace_id
  AND wi.service_name = nt.service_name
  AND wi.resource_path IS NOT NULL;
