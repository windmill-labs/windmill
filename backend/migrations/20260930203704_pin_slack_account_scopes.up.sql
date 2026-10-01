-- Accounts without stored scopes refresh with the provider's default scopes.
-- Slack's default bot scopes widened; a refresh asking for more than was granted
-- can fail, so pin these accounts to the scopes they were granted.
UPDATE account
SET scopes = ARRAY['chat:write', 'chat:write.public', 'channels:join', 'files:write']
WHERE client = 'slack'
  AND grant_type = 'authorization_code'
  AND (scopes IS NULL OR cardinality(scopes) = 0)
  -- An instance connect_config with a token URL replaces the registry entry, scopes
  -- included, so its accounts never refreshed with the registry default.
  AND NOT EXISTS (
    SELECT 1 FROM global_settings
    WHERE name = 'oauths'
      AND coalesce(value -> 'slack' -> 'connect_config' ->> 'token_url', '') <> ''
  );
