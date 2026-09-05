-- Column-level lineage, from the engine's own static analysis.
--
-- `manifest.json` carries none, which is why decision 14 recorded the feature as
-- unavailable. The edges exist in a different artifact: an engine that does
-- static analysis writes `target/index/dbt.column_lineage.parquet` under
-- `dbt compile --static-analysis strict --write-index`. That pass is opt-in per
-- project (`column_lineage: true`), because strict analysis rejects SQL the
-- default accepts and must never become a silent requirement of running a build.

-- One column-to-column edge, keyed exactly like `dbt_edge`: a version's graph
-- dies with its version through the composite foreign key, a run's snapshot is
-- keyed by `job_id` with the zero UUID meaning "the version's own graph", and an
-- editor buffer's parse carries a NULL `script_hash` keyed to its preview job.
CREATE TABLE IF NOT EXISTS dbt_column_edge (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path      VARCHAR(255) NOT NULL,
  script_hash      BIGINT,
  job_id           UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
  parent_unique_id TEXT NOT NULL,
  parent_column    TEXT NOT NULL,
  child_unique_id  TEXT NOT NULL,
  child_column     TEXT NOT NULL,
  -- dbt's own word for how the value travelled: `copy` (passthrough), `mod`
  -- (transformed), `scan` (the column was read to produce the ROW rather than
  -- the value -- a join key, a `where` predicate, a `group by`). TEXT rather
  -- than an enum because the engine treats the set as open: its own reader maps
  -- those three and returns anything else verbatim.
  lineage_kind     TEXT NOT NULL,
  ingested_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  -- Two partial unique indexes rather than a primary key, for the reason
  -- 20260801121717 gives: a versioned graph is keyed by its version, a buffer
  -- parse by its job alone.
  CONSTRAINT dbt_column_edge_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS dbt_column_edge_versioned_key
  ON dbt_column_edge (workspace_id, script_path, script_hash, job_id,
                      parent_unique_id, parent_column, child_unique_id, child_column)
  WHERE script_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_column_edge_editor_key
  ON dbt_column_edge (workspace_id, job_id,
                      parent_unique_id, parent_column, child_unique_id, child_column)
  WHERE script_hash IS NULL;

-- Same age sweep as the other per-run rows, and the same reason there is no
-- foreign key to `v2_job`.
CREATE INDEX IF NOT EXISTS idx_dbt_column_edge_run_age ON dbt_column_edge (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';

-- The real column schema of a node, which only static analysis knows: an
-- ordered `[{"name": …, "type": …}]`, from `dbt.node_columns.parquet`.
--
-- Beside `columns` rather than folded into it. `columns` is the DECLARED
-- metadata `manifest.json` carries -- the names an author wrote in `schema.yml`
-- and the prose against them -- and stays exactly that, so a project that
-- documents two of forty columns keeps saying so. This is the other forty,
-- typed, in the order the model produces them.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS column_schema JSONB;

GRANT ALL ON dbt_column_edge TO windmill_user;
GRANT ALL ON dbt_column_edge TO windmill_admin;
