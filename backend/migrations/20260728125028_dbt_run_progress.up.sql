-- Live per-model progress for one dbt RUN.
--
-- `materialized_partition` cannot answer this: its key is the relation, one row
-- per table, and its `job_id` names only the last writer. Two runs of one
-- project building the same models take that row from each other, so a progress
-- read filtered by job loses nodes and flickers between states. That table is
-- left exactly as it is — the current state of a relation is what the pipeline
-- canvas and fork defer read, and one row per relation is right for them.
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

-- No foreign key to `v2_job`: three were deliberately dropped from it for the
-- write amplification they cost on the hottest table in the system. Growth is
-- bounded by age instead and pruned by the runs themselves, so no background
-- sweep has to learn about this table.
CREATE INDEX IF NOT EXISTS idx_dbt_run_progress_updated_at
  ON dbt_run_progress (updated_at);

-- Written from a worker on a user-scoped transaction, like `dbt_node`, so the
-- grants have to be explicit rather than inherited from ALTER DEFAULT
-- PRIVILEGES (see 20260720131744).
GRANT ALL ON dbt_run_progress TO windmill_user;
GRANT ALL ON dbt_run_progress TO windmill_admin;
