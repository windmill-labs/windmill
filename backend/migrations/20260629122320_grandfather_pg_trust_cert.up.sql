-- TLS certificate verification is now enforced for verify-ca/verify-full Postgres
-- resources (previously these modes accepted any certificate when no root cert was
-- supplied). Grandfather existing resources whose behavior would otherwise change
-- in a way that could break the connection: verify-ca/verify-full with no root
-- certificate. These get trust_cert=true so they keep connecting unchanged.
-- Resources that already had a certificate were already verifying, and
-- require/disable are unaffected, so neither is touched. Newly created resources
-- default to verifying.
UPDATE resource
SET value = jsonb_set(value, '{trust_cert}', 'true'::jsonb, true)
WHERE resource_type = 'postgresql'
  AND jsonb_typeof(value) = 'object'
  AND NOT (value ? 'trust_cert')
  AND coalesce(value ->> 'sslmode', '') IN ('verify-ca', 'verify-full')
  AND coalesce(value ->> 'root_certificate_pem', '') = '';

-- Expose the trust_cert toggle in the resource editor on instances that already
-- imported the postgresql resource type (fresh imports pick it up from the hub).
UPDATE resource_type
SET schema = jsonb_set(
        schema,
        '{properties,trust_cert}',
        '{"type": "boolean", "default": false, "description": "Trust any certificate presented by the server (skips TLS certificate verification). Leave off to verify the certificate according to sslmode."}'::jsonb,
        true
    )
WHERE name = 'postgresql'
  AND jsonb_typeof(schema) = 'object'
  AND jsonb_typeof(schema -> 'properties') = 'object'
  AND NOT (schema -> 'properties' ? 'trust_cert');
