-- Accounts without stored scopes refresh with the provider's default scopes.
-- Gmail's default widened from gmail.send to gmail.modify; a refresh asking for
-- more than was granted fails, so pin these accounts to the scope they were granted.
UPDATE account
SET scopes = ARRAY['https://www.googleapis.com/auth/gmail.send']
WHERE client = 'gmail'
  AND grant_type = 'authorization_code'
  AND (scopes IS NULL OR cardinality(scopes) = 0)
  -- An instance connect_config with a URL replaces the registry entry, scopes included,
  -- so its accounts never refreshed with gmail.send.
  AND NOT EXISTS (
    SELECT 1 FROM global_settings
    WHERE name = 'oauths'
      AND (coalesce(value -> 'gmail' -> 'connect_config' ->> 'auth_url', '') <> ''
        OR coalesce(value -> 'gmail' -> 'connect_config' ->> 'token_url', '') <> '')
  );
