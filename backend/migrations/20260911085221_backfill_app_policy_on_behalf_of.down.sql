-- Add down migration script here
-- The up migration is additive: it gives a policy that only ever carried the address the
-- principal it runs as, and removes nothing. The previous version reads both halves, so the
-- backfilled principal is correct for it too and there is nothing to undo.
SELECT 1;
