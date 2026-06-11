-- WIN-2006: grandfather all apps that exist at upgrade time as "legacy unsandboxed"
-- so they keep running same-origin (their pre-sandbox behavior) without breaking and
-- without prompting viewers. Apps created after this migration are sandboxed by
-- default; a re-deploy of a legacy app clears the flag.
UPDATE app
SET policy = jsonb_set(policy, '{legacy_unsandboxed}', 'true'::jsonb)
WHERE NOT (policy ? 'legacy_unsandboxed');
