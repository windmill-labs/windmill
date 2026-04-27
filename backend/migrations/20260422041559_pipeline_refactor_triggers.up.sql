-- Simplified pipeline model: `// materialize` is a bare opt-in marker (sets
-- auto_kind='materializer') and the per-asset ownership tracking goes away.
-- Writes are already tracked in the `asset` table via the parser, so the
-- `asset_materializer*` tables no longer earn their keep.
DROP TABLE IF EXISTS asset_materializer_history;
DROP TABLE IF EXISTS asset_materializer;

-- Execution DAG edges declared via `// on <asset | schedule>` annotations.
-- For `trigger_kind='asset'`: trigger_ref is `<kind>://<path>` (kind from
--   parse_asset_syntax, so downstream lookups match the `asset` table).
-- For `trigger_kind='schedule'`: trigger_ref is the raw cron expression.
CREATE TYPE SCRIPT_TRIGGER_KIND AS ENUM ('asset', 'schedule');

CREATE TABLE script_trigger (
  id             BIGSERIAL PRIMARY KEY,
  workspace_id   VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  runnable_kind  ASSET_USAGE_KIND NOT NULL,
  runnable_path  VARCHAR(255) NOT NULL,
  trigger_kind   SCRIPT_TRIGGER_KIND NOT NULL,
  trigger_ref    TEXT NOT NULL
);

-- Per-runnable lookup (wipe-on-deploy, list-triggers-for-script).
CREATE INDEX idx_script_trigger_runnable
  ON script_trigger (workspace_id, runnable_kind, runnable_path);

-- Reverse lookup: "which scripts are triggered by asset X?" (the asset → script
-- edges in the graph). trigger_ref is unbounded text so can't share the
-- asset_kind btree, but this covers the common prefix-scan use case.
CREATE INDEX idx_script_trigger_ref
  ON script_trigger (workspace_id, trigger_kind, trigger_ref);
