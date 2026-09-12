-- The app policy's address is derived from its principal on every read and no longer written,
-- so a stored copy is inert and, once an account is renamed or its address changes, wrong.
UPDATE app SET policy = policy - 'on_behalf_of_email' WHERE policy ? 'on_behalf_of_email';

UPDATE draft SET value = to_json(to_jsonb(value) #- '{policy,on_behalf_of_email}')
WHERE typ IN ('app', 'raw_app') AND to_jsonb(value) #> '{policy}' ? 'on_behalf_of_email';
