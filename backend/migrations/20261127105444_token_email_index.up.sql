-- Serves every per-user token listing (`list_tokens`, the token risk counts), which otherwise
-- scans the whole table, sessions and job tokens included.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS token_email_idx;

CREATE INDEX IF NOT EXISTS token_email_idx ON token (email);
