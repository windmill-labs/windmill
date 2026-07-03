-- Attribution for runs pushed by the pipeline freshness watchdog (the EE
-- background loop that re-runs a `// freshness`-annotated producer whose
-- output aged past its window).
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'freshness';

-- Per-(workspace, script) watchdog state: exponential-backoff bookkeeping so
-- a persistently failing producer isn't re-pushed on every scan tick, and an
-- atomic claim so concurrent servers can't double-push in the same tick
-- (claim = the UPDATE/INSERT that advances next_attempt_at; only the winner
-- pushes). Rows exist only while a script is stale — observing it fresh (or
-- its annotation gone) deletes the row, resetting the backoff.
CREATE TABLE pipeline_freshness_state (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    script_path VARCHAR(510) NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 1,
    last_push_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, script_path)
);

-- Written only by the server monitor loop on the raw (non-RLS) pool, but
-- granted like every other app table so a future user-transaction reader
-- doesn't hit the recurring missing-GRANT class of bug.
GRANT ALL ON pipeline_freshness_state TO windmill_user;
GRANT ALL ON pipeline_freshness_state TO windmill_admin;
