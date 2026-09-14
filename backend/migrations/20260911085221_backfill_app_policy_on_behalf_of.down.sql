-- Add down migration script here
-- Nothing to undo. The up migration gives a policy that only ever carried the address the
-- principal it runs as, and rewrites an address that disagreed with its principal. The previous
-- version reads both halves, so both results are correct for it too, and the addresses replaced
-- named an account other than the one the app runs as.
SELECT 1;
