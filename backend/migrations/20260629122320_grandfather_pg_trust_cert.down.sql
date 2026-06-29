-- Remove the grandfather flag. Scoped to the same set the up migration stamped
-- (verify-ca/verify-full, no root cert) to limit clobbering a flag an operator
-- may have set by hand.
UPDATE resource
SET value = value - 'trust_cert'
WHERE resource_type = 'postgresql'
  AND jsonb_typeof(value) = 'object'
  AND value ->> 'trust_cert' = 'true'
  AND coalesce(value ->> 'sslmode', '') IN ('verify-ca', 'verify-full')
  AND coalesce(value ->> 'root_certificate_pem', '') = '';
