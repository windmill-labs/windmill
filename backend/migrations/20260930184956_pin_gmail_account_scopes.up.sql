-- Accounts without stored scopes refresh with the provider's default scopes.
-- Gmail's default widened from gmail.send to gmail.modify; a refresh asking for
-- more than was granted fails, so pin these accounts to the scope they were granted.
UPDATE account
SET scopes = ARRAY['https://www.googleapis.com/auth/gmail.send']
WHERE client = 'gmail'
  AND grant_type = 'authorization_code'
  AND (scopes IS NULL OR cardinality(scopes) = 0);
