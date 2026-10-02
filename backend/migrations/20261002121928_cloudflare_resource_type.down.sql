-- Nothing to undo: the hub publishes the same `cloudflare` type, so the row may not be this
-- migration's, and other Cloudflare scripts rely on it.
SELECT 1;
