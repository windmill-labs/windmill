-- TLS certificate verification is now enforced for verify-ca/verify-full Postgres
-- resources (previously these modes accepted any certificate when no root cert was
-- supplied). Grandfather existing resources whose behavior would otherwise change
-- in a way that could break the connection: verify-ca/verify-full with no root
-- certificate. These get trust_cert=true so they keep connecting unchanged.
-- Resources that already had a certificate were already verifying, and
-- require/disable are unaffected, so neither is touched. Newly created resources
-- default to verifying.
-- The trust_cert property itself is defined on the hub postgresql resource type;
-- this migration only stamps the existing data so the upgrade is non-breaking.
UPDATE resource
SET value = jsonb_set(value, '{trust_cert}', 'true'::jsonb, true)
WHERE resource_type = 'postgresql'
  AND jsonb_typeof(value) = 'object'
  AND NOT (value ? 'trust_cert')
  AND coalesce(value ->> 'sslmode', '') IN ('verify-ca', 'verify-full')
  AND coalesce(value ->> 'root_certificate_pem', '') = '';
