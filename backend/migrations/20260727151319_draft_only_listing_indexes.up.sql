-- Index backing draft-only rows in the unified homepage listing
-- (/runnables/list) and its per-owner counts (/runnables/counts).
--
-- Every draft query there is "this caller's drafts of this kind", so `typ` and
-- `email` are key columns rather than filters: without them the existing
-- draft_user_listing_idx hands back the caller's drafts of every kind and each
-- per-kind branch throws most away.
--
-- Deliberately just these three columns. `path` would not help: a draft is
-- listed, grouped and filtered under the path it says it will deploy to, which
-- lives in the draft JSON and is only resolved after the DISTINCT ON dedup, so
-- no index can prune on it. Nor can "draft-only" itself be pushed into the
-- index — it means "no deployed row at this path", and Postgres rejects a
-- subquery in an index predicate; the anti-join prunes it at query time off the
-- deployed tables' own (workspace_id, path) indexes.
-- Created CONCURRENTLY via the OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs.
CREATE INDEX IF NOT EXISTS draft_kind_user_listing_idx
    ON draft (workspace_id, typ, email);
