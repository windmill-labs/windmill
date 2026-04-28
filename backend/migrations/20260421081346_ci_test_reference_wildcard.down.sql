DROP INDEX IF EXISTS idx_ci_test_ref_wildcards;
ALTER TABLE ci_test_reference DROP COLUMN IF EXISTS has_wildcard;
