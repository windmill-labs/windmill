-- Fix triggers/schedules where permissioned_as = 'u/{username}' but the user
-- does not exist in the workspace's usr table.  Replace with the raw email
-- from the instance-level password table so the super-admin check succeeds.

UPDATE http_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE websocket_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE postgres_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE mqtt_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE kafka_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE nats_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE sqs_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE gcp_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE email_trigger t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );

UPDATE schedule t SET permissioned_as = p.email
FROM password p
WHERE t.permissioned_as LIKE 'u/%'
  AND p.username = SUBSTRING(t.permissioned_as FROM 3)
  AND NOT EXISTS (
      SELECT 1 FROM usr u
      WHERE u.username = SUBSTRING(t.permissioned_as FROM 3)
        AND u.workspace_id = t.workspace_id
  );
