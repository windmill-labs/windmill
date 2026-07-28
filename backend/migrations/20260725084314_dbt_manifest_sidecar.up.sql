-- Physical-relation asset kind, shared by dbt models and any other runtime that
-- writes a warehouse table. Identity is the relation itself
-- (`<resource_path>/<schema>/<name>`), never the producing tool, so a dbt mart
-- and a native script reading the same table resolve to one node and the
-- lineage is one graph across the boundary (docs/dbt-runtime.md, decision 11).
ALTER TYPE ASSET_KIND ADD VALUE IF NOT EXISTS 'table';

-- Parsed dbt manifest, one row per dbt node, keyed by the script that owns the
-- dbt project. Written by the deploy-time dependency job (pinned refs) or by
-- the run itself (`ref: latest`), wiped and reinserted per ingest. The full
-- manifest.json is not stored here -- only the fields the asset graph renders.
CREATE TABLE IF NOT EXISTS dbt_node (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path      VARCHAR(255) NOT NULL,
  -- dbt's own node id, e.g. `model.jaffle_shop.customers`. Stable across runs
  -- and the join key for dbt_edge and for run_results ingestion.
  unique_id        TEXT NOT NULL,
  resource_type    TEXT NOT NULL,
  name             TEXT NOT NULL,
  -- The physical relation this node produces, spelled as the matching `asset`
  -- row's path (`<resource_path>/<schema>/<name>`). NULL for nodes with no
  -- relation: ephemeral models, tests.
  asset_path       TEXT,
  materialized     TEXT,
  -- Windmill's equivalent write strategy (replace | append | merge | scd2),
  -- NULL when dbt's materialization has no analogue (view, ephemeral).
  materialize_strategy TEXT,
  unique_key       TEXT,
  tags             TEXT[] NOT NULL DEFAULT '{}',
  description      TEXT,
  -- Test nodes only: the generic test name (`unique`, `not_null`,
  -- `accepted_values`, `relationships`, or a package/custom test's own name),
  -- the column it is attached to, its rendered kwargs and dbt's severity.
  -- Severity is stored as dbt spells it; readers compare case-insensitively
  -- because dbt-core 1.x echoes the author's casing while 2.x uppercases it.
  test_kind        TEXT,
  test_column      TEXT,
  test_args        JSONB,
  severity         TEXT,
  attached_node    TEXT,
  -- Declared per-column metadata (name -> description), the column sets the
  -- asset graph shows. dbt's manifest carries no column-to-column lineage, so
  -- this is not a column lineage graph.
  columns          JSONB,
  freshness        JSONB,
  PRIMARY KEY (workspace_id, script_path, unique_id)
);

-- `ref()` / `source()` lineage, straight from the manifest's parent_map.
CREATE TABLE IF NOT EXISTS dbt_edge (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path      VARCHAR(255) NOT NULL,
  parent_unique_id TEXT NOT NULL,
  child_unique_id  TEXT NOT NULL,
  PRIMARY KEY (workspace_id, script_path, parent_unique_id, child_unique_id)
);

-- Resolve a physical relation back to the dbt node that produces it: the asset
-- graph renders from `asset` rows and needs the dbt provenance per node.
CREATE INDEX IF NOT EXISTS idx_dbt_node_asset_path
  ON dbt_node (workspace_id, asset_path) WHERE asset_path IS NOT NULL;

-- Both tables are written on a user_db transaction (SET LOCAL ROLE
-- windmill_user/windmill_admin), so they must be granted explicitly rather than
-- relying on ALTER DEFAULT PRIVILEGES, which only covers objects created by the
-- role that set them (see 20260720131744).
GRANT ALL ON dbt_node TO windmill_user;
GRANT ALL ON dbt_node TO windmill_admin;
GRANT ALL ON dbt_edge TO windmill_user;
GRANT ALL ON dbt_edge TO windmill_admin;
