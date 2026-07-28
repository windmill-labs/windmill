-- One row per stored graph, so a snapshot's EXISTENCE does not depend on it
-- having any nodes.
--
-- A dynamic descriptor can disable every model for a run; that run's graph is
-- legitimately empty, and inferring existence from a `dbt_node` row made it
-- indistinguishable from a run that stored nothing — the page then fell back to
-- the deployed graph and showed models the run never built.
--
-- It also gives the content digest one home. It was a column on every node,
-- repeated per row and read back with a `LIMIT 1` that had no `job_id`
-- predicate, so a run could compare itself against another run's digest.
CREATE TABLE IF NOT EXISTS dbt_graph_snapshot (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path  VARCHAR(255) NOT NULL,
  script_hash  BIGINT NOT NULL,
  -- The zero UUID is the version's own graph, as deployed; anything else is the
  -- snapshot one run took.
  job_id       UUID NOT NULL,
  digest       TEXT NOT NULL,
  ingested_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id)
);

-- The graph queries ask "does this job have a snapshot", and the sweep walks by
-- age across every workspace.
CREATE INDEX IF NOT EXISTS idx_dbt_graph_snapshot_job
  ON dbt_graph_snapshot (workspace_id, job_id);
CREATE INDEX IF NOT EXISTS idx_dbt_graph_snapshot_age
  ON dbt_graph_snapshot (ingested_at)
  WHERE job_id <> '00000000-0000-0000-0000-000000000000';

GRANT ALL ON dbt_graph_snapshot TO windmill_user;
GRANT ALL ON dbt_graph_snapshot TO windmill_admin;

-- Backfill so existing graphs keep their identity rather than looking absent.
INSERT INTO dbt_graph_snapshot (workspace_id, script_path, script_hash, job_id, digest)
SELECT DISTINCT workspace_id, script_path, script_hash, job_id,
       COALESCE(graph_digest, '')
  FROM dbt_node
ON CONFLICT DO NOTHING;

ALTER TABLE dbt_node DROP COLUMN IF EXISTS graph_digest;
