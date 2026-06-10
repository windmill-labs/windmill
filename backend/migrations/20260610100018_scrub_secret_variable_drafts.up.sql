-- Secret variable values used to be autosaved into `draft.value` in
-- plaintext (deployed secrets are encrypted with the workspace crypt key
-- precisely so DB dumps don't leak them). `save_draft` now blanks
-- `variable.value` for `is_secret: true` drafts at write time; this
-- scrubs the rows persisted before that guard existed.
UPDATE draft
SET value = jsonb_set(value::jsonb, '{variable,value}', '""'::jsonb)::json
WHERE typ = 'variable'
  AND (value::jsonb -> 'variable' ->> 'is_secret')::boolean IS TRUE
  AND value::jsonb -> 'variable' ? 'value';
