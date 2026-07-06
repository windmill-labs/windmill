-- Marks an asset-cascade edge that was auto-derived from a `// pipeline`
-- script's ducklake/s3 read (vs an explicit `// on <asset>`). Purely
-- informational — the dispatcher treats derived and explicit edges
-- identically; the graph endpoint returns it so the canvas can render
-- auto-wired edges distinctly from declared ones. Existing rows predate
-- derivation, so FALSE (explicit) is the correct backfill default.
ALTER TABLE script_trigger ADD COLUMN IF NOT EXISTS derived BOOLEAN NOT NULL DEFAULT FALSE;
