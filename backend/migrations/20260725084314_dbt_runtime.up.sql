-- The dbt runtime's tables: the parsed manifest that becomes the asset graph,
-- the retry state a `dbt retry` resumes from, and a run's live per-model
-- progress.

-- Physical-relation asset kind. Identity is the relation itself
-- (`<warehouse>/<schema>/<name>`, the warehouse named as the workspace
-- configures it), never the producing tool, so a dbt mart and a native script
-- reading the same table resolve to one node and the lineage is one graph
-- across the boundary (docs/dbt-runtime.md, decision 11).
ALTER TYPE ASSET_KIND ADD VALUE IF NOT EXISTS 'dbt';

-- Warehouses configured once for the workspace, so a dbt project needs no
-- connection knowledge to run: the descriptor names one by NAME (or nothing, for
-- `main`) and the value here points at the RESOURCE that holds the credentials,
-- exactly as `large_file_storage` does for buckets. A flat map keyed by name,
-- which is also what asset identity keys on.
--
--   {"main": {"resource_path": "$res:u/admin/wh", "target": "prod"},
--    "eu":   {"resource_path": "$res:u/admin/wh_eu"}}
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS dbt_warehouses JSONB;

-- Parsed dbt manifest, one row per dbt node. The full manifest.json is not
-- stored -- only the fields the asset graph renders.
--
-- Keyed by (path, version, job). The VERSION because each deployed version of a
-- script keeps its own graph, so a finished run can be shown the project as it
-- was rather than whatever is deployed today. The JOB because a dynamic
-- descriptor -- a `{{ }}` placeholder in `vars` -- can resolve to a different
-- set of models per run, and those runs re-ingest; the zero UUID means "the
-- version's own graph, as deployed", which is what a static descriptor writes
-- once and every one of its runs reads. A sentinel rather than NULL because
-- `job_id` is part of the key and Postgres does not treat two NULLs as the same
-- key, so each re-ingest would add a row set instead of replacing one.
CREATE TABLE IF NOT EXISTS dbt_node (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path      VARCHAR(255) NOT NULL,
  script_hash      BIGINT NOT NULL,
  job_id           UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
  -- dbt's own node id, e.g. `model.jaffle_shop.customers`. Stable across runs
  -- and the join key for dbt_edge and for run_results ingestion.
  unique_id        TEXT NOT NULL,
  resource_type    TEXT NOT NULL,
  name             TEXT NOT NULL,
  -- The physical relation this node produces, spelled as the matching `asset`
  -- row's path (`<warehouse>/<schema>/<name>`). NULL for nodes with no
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
  -- The model's SQL as written, for the graph to render. `dbt parse` fills it;
  -- `compiled_code` would need a `dbt compile`, which no phase runs.
  raw_code         TEXT,
  -- Its path inside the dbt project, e.g. `models/staging/stg_orders.sql`.
  original_file_path TEXT,
  ingested_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id, unique_id),
  -- Composite because `script`'s key is (workspace_id, hash). A version's graph
  -- dies with the version and nothing has to sweep it.
  CONSTRAINT dbt_node_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE ON UPDATE CASCADE
);

-- `ref()` / `source()` lineage, straight from the manifest's parent_map.
CREATE TABLE IF NOT EXISTS dbt_edge (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path      VARCHAR(255) NOT NULL,
  script_hash      BIGINT NOT NULL,
  job_id           UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
  parent_unique_id TEXT NOT NULL,
  child_unique_id  TEXT NOT NULL,
  ingested_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id, parent_unique_id, child_unique_id),
  CONSTRAINT dbt_edge_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE ON UPDATE CASCADE
);

-- One row per stored graph, so a snapshot's EXISTENCE does not depend on it
-- having any nodes: a dynamic run can disable every model, and that empty graph
-- has to be distinguishable from a run that stored nothing -- otherwise the run
-- page falls back to the deployed models and shows what the run never built.
--
-- It is also where the content digest lives, once. A run whose digest matches
-- the version's writes no snapshot at all: marking a descriptor dynamic is
-- conservative (a `{{ }}` in `vars` says the arguments reach dbt, not that they
-- change which models exist), so the usual dynamic run resolves to exactly the
-- graph the deploy stored.
CREATE TABLE IF NOT EXISTS dbt_graph_snapshot (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path  VARCHAR(255) NOT NULL,
  script_hash  BIGINT NOT NULL,
  job_id       UUID NOT NULL,
  digest       TEXT NOT NULL,
  -- On the DEPLOYED row only: the relation root the last ingest to write this
  -- row resolved. Written by every ingest that is not a run's own snapshot,
  -- including one that publishes no ownership — a version that cannot claim the
  -- path would otherwise record nothing and compare against a stale root.
  --
  -- The drift check needs "where do the current usages point", and no other
  -- row answers it: the deploy's own root goes stale the moment a run at a
  -- moved profile republishes, and "the newest ingest" is wrong because a run
  -- whose graph matches the deploy's stores nothing at all.
  relation_root_at_last_ingest TEXT,
  ingested_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id),
  -- Same cascade as the rows it stands for. A marker outliving its nodes is
  -- read as a snapshot that has none, and its digest still answers the
  -- suppression check.
  CONSTRAINT dbt_graph_snapshot_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE ON UPDATE CASCADE
);

-- What a `dbt retry` resumes from. Keyed by path, not by version: there is one
-- saved run per script, and `identity` -- the project digest, warehouse and
-- engine -- is what refuses a resume that no longer describes the same run.
CREATE TABLE IF NOT EXISTS dbt_run_state (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path  VARCHAR(255) NOT NULL,
  -- Part of the key: a retry replaces the caller's arguments with the saved
  -- ones, so state written by one principal must not be restorable by another
  -- — that would hand them the literal `select` and `vars` of a run they were
  -- never entitled to see.
  permissioned_as VARCHAR(255) NOT NULL,
  identity     TEXT NOT NULL,
  -- The invocation's job arguments. `dbt retry` reuses the original selection
  -- and vars, so the graph refresh and the build must agree with them rather
  -- than with the retry request's.
  args         JSONB NOT NULL DEFAULT '{}'::jsonb,
  -- Text rather than jsonb: nothing queries inside it, and this is highly
  -- repetitive JSON that TOAST's compression handles well (about nine to one).
  run_results  TEXT NOT NULL,
  job_id       UUID,
  -- Whether those results hold a node `dbt retry` would rebuild. A run that
  -- failed before building anything, or succeeded outright, is still saved —
  -- restoring it is how a retry can say the run succeeded rather than that no
  -- state exists — but nothing may offer it as resumable.
  retryable    BOOLEAN NOT NULL DEFAULT false,
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, permissioned_as)
);

-- Live per-model progress for one RUN.
--
-- `materialized_partition` cannot answer this: its key is the relation, one row
-- per table, and its `job_id` names only the last writer. Two runs of one
-- project building the same models take that row from each other, so a progress
-- read filtered by job loses nodes and flickers between states. That table is
-- left as it is -- the current state of a relation is what the pipeline canvas
-- and fork defer read, and one row per relation is right for them.
CREATE TABLE IF NOT EXISTS dbt_run_progress (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  job_id       UUID NOT NULL,
  asset_kind   ASSET_KIND NOT NULL,
  asset_path   VARCHAR(255) NOT NULL,
  status       MATERIALIZATION_STATUS NOT NULL,
  row_count    BIGINT,
  error        TEXT,
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, job_id, asset_kind, asset_path)
);

-- Resolve a physical relation back to the dbt node that produces it: the asset
-- graph renders from `asset` rows and needs the dbt provenance per node.
CREATE INDEX IF NOT EXISTS idx_dbt_node_asset_path
  ON dbt_node (workspace_id, asset_path) WHERE asset_path IS NOT NULL;

-- No foreign key from the per-run rows to `v2_job`: three were deliberately
-- dropped from it for the write amplification they cost on the hottest table in
-- the system. Growth is bounded by age instead and pruned by the dbt runs
-- themselves, so no background sweep has to learn about these tables -- hence
-- an index per age sweep, and one per "does this job have a snapshot" lookup.
CREATE INDEX IF NOT EXISTS idx_dbt_node_job ON dbt_node (job_id)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_node_run_age ON dbt_node (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_edge_run_age ON dbt_edge (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_graph_snapshot_job
  ON dbt_graph_snapshot (workspace_id, job_id);
CREATE INDEX IF NOT EXISTS idx_dbt_graph_snapshot_age
  ON dbt_graph_snapshot (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_run_progress_updated_at
  ON dbt_run_progress (updated_at);

-- All of these are written on a user_db transaction (SET LOCAL ROLE
-- windmill_user/windmill_admin), so they must be granted explicitly rather than
-- relying on ALTER DEFAULT PRIVILEGES, which only covers objects created by the
-- role that set them (see 20260720131744).
GRANT ALL ON dbt_node TO windmill_user;
GRANT ALL ON dbt_node TO windmill_admin;
GRANT ALL ON dbt_edge TO windmill_user;
GRANT ALL ON dbt_edge TO windmill_admin;
GRANT ALL ON dbt_graph_snapshot TO windmill_user;
GRANT ALL ON dbt_graph_snapshot TO windmill_admin;
GRANT ALL ON dbt_run_state TO windmill_user;
GRANT ALL ON dbt_run_state TO windmill_admin;
GRANT ALL ON dbt_run_progress TO windmill_user;
GRANT ALL ON dbt_run_progress TO windmill_admin;
