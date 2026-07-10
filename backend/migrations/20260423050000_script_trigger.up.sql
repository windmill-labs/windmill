-- Execution DAG edges declared via `// on <asset | schedule | ...>`
-- annotations.
-- For `trigger_kind='asset'`: trigger_ref is `<kind>://<path>` (kind from
--   parse_asset_syntax, so downstream lookups match the `asset` table).
-- The other kinds mirror the keywords the annotation parser recognises in
-- `// on <kind> <ref>` lines (every non-integration trigger kind; their
-- trigger_ref is the trigger row path, or empty for marker-only forms).
--
-- Per-edge columns that are in fact script-level properties (every row for
-- a given runnable carries the same value, set once at deploy) but live on
-- the edge so the dispatcher reads everything from a single query:
--   join_all      `// trigger all` AND-join barrier (else OR, the default).
--   debounce_s    `// on debounce=` (else script-level `// debounce`); only
--                 asset-cascade edges carry one. NULL = no debounce.
--   retry_*       `// retry <count> [<delay>]` cascade retry. NULL = none.
--
-- The idempotency guards (IF NOT EXISTS / duplicate_object) are load-bearing:
-- this migration squashes several pre-release ones, so databases migrated
-- from the unsquashed history already contain the final objects and
-- re-applying must be a no-op.
DO $$ BEGIN
    CREATE TYPE SCRIPT_TRIGGER_KIND AS ENUM (
        'asset', 'schedule', 'webhook', 'email', 'kafka', 'mqtt', 'nats',
        'postgres', 'sqs', 'gcp');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS script_trigger (
  id             BIGSERIAL PRIMARY KEY,
  workspace_id   VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  runnable_kind  ASSET_USAGE_KIND NOT NULL,
  runnable_path  VARCHAR(255) NOT NULL,
  trigger_kind   SCRIPT_TRIGGER_KIND NOT NULL,
  trigger_ref    TEXT NOT NULL,
  join_all       BOOLEAN NOT NULL DEFAULT FALSE,
  debounce_s     INTEGER,
  retry_count    SMALLINT,
  retry_delay_s  INTEGER
);

-- Per-runnable lookup (wipe-on-deploy, list-triggers-for-script).
CREATE INDEX IF NOT EXISTS idx_script_trigger_runnable
  ON script_trigger (workspace_id, runnable_kind, runnable_path);

-- Reverse lookup: "which scripts are triggered by asset X?" (the asset → script
-- edges in the graph). trigger_ref is unbounded text so can't share the
-- asset_kind btree, but this covers the common prefix-scan use case.
CREATE INDEX IF NOT EXISTS idx_script_trigger_ref
  ON script_trigger (workspace_id, trigger_kind, trigger_ref);

-- Fast lookups for:
--   1. "does folder F have a pipeline?"  (exists check on prefix)
--   2. "list all folders with a pipeline" (distinct folder from path)
-- The partial predicate keeps the index tiny on workspaces with few
-- pipeline scripts, and text_pattern_ops lets 'f/foo/%' LIKE scans use it.
CREATE INDEX IF NOT EXISTS idx_script_pipeline_path
  ON script (workspace_id, path text_pattern_ops)
  WHERE auto_kind = 'pipeline' AND archived = false AND deleted = false;
