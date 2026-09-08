-- Indexes backing the homepage tree's per-owner runnable counts
-- (/runnables/counts), which counts each `f/<folder>` / `u/<user>` prefix with a
-- byte-ordered range `path ~>=~ owner || '/' AND path ~<~ owner || '0'`.
-- text_pattern_ops is required: the default opclass sorts by the database
-- collation, so a byte-prefix range cannot use it. `auto_kind` is an INCLUDE
-- column so the library-script filter is answered from the index tuple and the
-- count stays an index-only scan.
-- Created CONCURRENTLY via the OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs.
CREATE INDEX IF NOT EXISTS idx_script_owner_prefix
    ON script (workspace_id, path text_pattern_ops)
    INCLUDE (auto_kind)
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS idx_flow_owner_prefix
    ON flow (workspace_id, path text_pattern_ops)
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS idx_app_owner_prefix
    ON app (workspace_id, path text_pattern_ops);

-- Same purpose as the existing script_extra_perms / flow_extra_perms GIN
-- indexes: the counts endpoint finds items shared explicitly with the caller
-- (or one of their groups) with a single `extra_perms ?| ARRAY[...]` scan.
CREATE INDEX IF NOT EXISTS app_extra_perms ON app USING gin (extra_perms);
