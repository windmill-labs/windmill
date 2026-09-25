-- Each native trigger names the connection (OAuth resource) it acts through, instead of every
-- trigger of a service sharing the single workspace connection.
ALTER TABLE native_trigger ADD COLUMN connection_path VARCHAR(255);

UPDATE native_trigger nt
SET connection_path = wi.resource_path
FROM workspace_integrations wi
WHERE wi.workspace_id = nt.workspace_id
  AND wi.service_name = nt.service_name
  AND wi.resource_path IS NOT NULL;
