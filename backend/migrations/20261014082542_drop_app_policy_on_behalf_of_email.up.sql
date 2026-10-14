-- The app policy's address is derived from its principal on every read and no longer written,
-- so a stored copy is inert and, once an account is renamed or its address changes, wrong.
--
-- Only where there is a principal to derive from. `20260911085221` left `on_behalf_of` unset
-- wherever the address named nobody or resolved to something too wide to enqueue; for those rows
-- the address is the sole surviving record of the intended identity and nothing can recompute it.
-- They already fail `get_on_behalf_of` on the missing principal, so keeping it costs nothing.
--
-- Tested through `->>` rather than `?`: a policy `Policy` wrote serializes an absent principal as
-- an explicit JSON null, which `?` counts as present.
UPDATE app SET policy = policy - 'on_behalf_of_email'
WHERE policy ? 'on_behalf_of_email' AND policy->>'on_behalf_of' IS NOT NULL;

UPDATE draft SET value = to_json(to_jsonb(value) #- '{policy,on_behalf_of_email}')
WHERE typ IN ('app', 'raw_app')
  AND to_jsonb(value) #> '{policy}' ? 'on_behalf_of_email'
  AND value->'policy'->>'on_behalf_of' IS NOT NULL;
