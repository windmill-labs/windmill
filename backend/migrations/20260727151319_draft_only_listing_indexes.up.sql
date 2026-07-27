-- Index backing draft-only rows in the unified homepage listing
-- (/runnables/list) and its per-owner counts (/runnables/counts).
--
-- Every draft query there is "this caller's drafts of this kind", so `typ` and
-- `email` are key columns rather than filters: without them the existing
-- draft_user_listing_idx hands back the caller's drafts of every kind and the
-- branch throws most away (measured on a 5k-draft user: 1.4ms vs 4.1ms per
-- kind). `path` sorts with text_pattern_ops so an owner-filtered listing
-- (`path LIKE 'f/foo/%'`) can seek instead of scanning the whole slice — the
-- default opclass sorts by the database collation and cannot serve a byte
-- prefix.
--
-- Deliberately the only index added: `draft` is written on every editor
-- autosave, and time/name sort indexes measured as pure overhead — the
-- listing's DISTINCT ON (path) dedup has to sort by path before the sort key
-- is applied, so an ordered index is never used for the order.
-- Created CONCURRENTLY via the OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs.
CREATE INDEX IF NOT EXISTS draft_kind_user_listing_idx
    ON draft (workspace_id, typ, email, path text_pattern_ops);
