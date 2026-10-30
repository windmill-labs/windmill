-- Guest apps are now gated by their execution mode alone. An app stored as `guest` where the
-- workspace switch was off, or the instance switch was set, admitted no guest; leave it
-- admitting none by moving it to `publisher`, which runs the same way for members.
UPDATE app
SET policy = jsonb_set(policy, '{execution_mode}', '"publisher"')
FROM workspace_settings ws
WHERE ws.workspace_id = app.workspace_id
  AND app.policy->>'execution_mode' = 'guest'
  AND (
    NOT ws.guest_access_enabled
    OR COALESCE(
      (SELECT (value #>> '{}')::boolean FROM global_settings WHERE name = 'guest_access_disabled'),
      false
    )
  );
