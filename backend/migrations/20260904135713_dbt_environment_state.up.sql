-- The dbt state one project last built into one environment: the `manifest.json`
-- (and the `run_results.json` beside it) that `dbt --defer --state <dir>` resolves
-- an unbuilt `ref()` through.
--
-- Separate from `dbt_run_state`, which answers a different question. That one is
-- keyed by the executing principal and holds the LAST run whatever its outcome,
-- so `dbt retry` can resume its failures; this one is keyed by environment and
-- holds the last SUCCESSFUL run, because a relation a later run defers to has to
-- exist. Merging them would make a retry resume a run that is not the last one,
-- or a deferral point at relations a failed run never wrote.
CREATE TABLE IF NOT EXISTS dbt_environment_state (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path  VARCHAR(255) NOT NULL,
  -- The workspace warehouse, the dbt target, and the database and schema that
  -- target resolves to. All four, because deferring is resolving a relation
  -- NAME: a repointed warehouse or a moved schema makes the stored manifest
  -- describe relations that are not where this run would look for them, and the
  -- run has no other way to notice. A move therefore reads as an environment
  -- with no state yet rather than as state that silently no longer fits.
  --
  -- TEXT rather than VARCHAR(255): a project bringing its own `profiles.yml`
  -- spells its own schema and database, so the length is the project's.
  environment  TEXT NOT NULL,
  -- The run that published it, so a deferring run can say what it deferred to.
  job_id       UUID NOT NULL,
  -- Exactly one home each. A manifest grows with the project and passes a few
  -- hundred KB on a handful of models, so a large one goes to the workspace's
  -- object storage and this row keeps the key; a small one stays here, where it
  -- costs no round trip and works on a workspace that has configured no storage
  -- at all. `run_results.json` is a tenth of the size and takes the same two
  -- homes rather than a rule of its own.
  manifest        TEXT,
  manifest_key    TEXT,
  run_results     TEXT,
  run_results_key TEXT,
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path, environment),
  CONSTRAINT dbt_environment_state_manifest_one_home
    CHECK (num_nonnulls(manifest, manifest_key) = 1),
  CONSTRAINT dbt_environment_state_run_results_one_home
    CHECK (num_nonnulls(run_results, run_results_key) <= 1)
);

-- No age sweep, unlike the per-run graph rows next door: this table holds one
-- row per script per environment and replaces it in place, so it does not grow
-- with runs, and its reader is every later run of that script — a project that
-- runs monthly must still find last month's state. It goes with the script
-- instead, alongside `dbt_run_state`.
GRANT ALL ON dbt_environment_state TO windmill_user;
GRANT ALL ON dbt_environment_state TO windmill_admin;
