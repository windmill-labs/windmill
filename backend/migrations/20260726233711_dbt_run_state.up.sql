-- Durable `dbt retry` state, one row per dbt script.
--
-- The previous run's `run_results.json` is what `dbt retry` selects from, and it
-- used to live only in the worker's local cache, so a retry that landed on
-- another worker of the same group found nothing.
--
-- Only `run_results.json` is stored. `dbt retry` also needs `manifest.json`,
-- which is roughly sixty times larger and grows with the project, but it is a
-- pure function of the project files, vars and env -- all of which `identity`
-- already pins -- so the resuming worker re-derives it with a `dbt parse`
-- (about a second) instead of keeping a copy per run.
CREATE TABLE IF NOT EXISTS dbt_run_state (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  script_path  VARCHAR(255) NOT NULL,
  -- Project digest, warehouse and engine: everything that decides which
  -- relations the saved results describe. A retry whose identity differs is
  -- refused, because those failures do not describe the run being retried.
  identity     TEXT NOT NULL,
  -- The invocation's job arguments. `dbt retry` reuses the original selection
  -- and vars, so the graph refresh and the build must agree with them rather
  -- than with the retry request's.
  args         JSONB NOT NULL DEFAULT '{}'::jsonb,
  -- Text rather than jsonb: nothing queries inside it, and this is highly
  -- repetitive JSON that TOAST's compression handles well (about nine to one).
  run_results  TEXT NOT NULL,
  job_id       UUID,
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, script_path)
);

GRANT ALL ON dbt_run_state TO windmill_user;
GRANT ALL ON dbt_run_state TO windmill_admin;
