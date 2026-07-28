-- A dynamic descriptor's graph belongs to the RUN, not just to the version.
--
-- A `{{ }}` placeholder in `vars` (or a `$var:` env) can enable a different set
-- of models per run, so those runs re-ingest. Keyed only by version, each
-- re-ingest overwrote the last: reopening an older run showed the newer run's
-- project, and any model only the older run built was gone — no SQL, no
-- lineage, nothing the saved result could colour.
--
-- `job_id` joins the key. A zero UUID means "the version's own graph, as
-- deployed", which is what a static descriptor writes once and every one of its
-- runs reads. A real job id is a snapshot written by that run, read back by its
-- own page, and pruned by age.
ALTER TABLE dbt_node
  ADD COLUMN IF NOT EXISTS job_id UUID NOT NULL
    DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE dbt_edge
  ADD COLUMN IF NOT EXISTS job_id UUID NOT NULL
    DEFAULT '00000000-0000-0000-0000-000000000000';

-- When the snapshot was taken, which is all the prune needs: a version's own
-- graph is never pruned, so this only ever ages out per-run rows.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS ingested_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE dbt_edge ADD COLUMN IF NOT EXISTS ingested_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE dbt_node DROP CONSTRAINT IF EXISTS dbt_node_pkey;
ALTER TABLE dbt_node
  ADD PRIMARY KEY (workspace_id, script_path, script_hash, job_id, unique_id);
ALTER TABLE dbt_edge DROP CONSTRAINT IF EXISTS dbt_edge_pkey;
ALTER TABLE dbt_edge
  ADD PRIMARY KEY (workspace_id, script_path, script_hash, job_id,
                   parent_unique_id, child_unique_id);

-- The prune walks by age across every workspace, and the run page reads one
-- job's snapshot directly.
CREATE INDEX IF NOT EXISTS idx_dbt_node_job ON dbt_node (job_id)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_node_run_age ON dbt_node (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
CREATE INDEX IF NOT EXISTS idx_dbt_edge_run_age ON dbt_edge (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';
