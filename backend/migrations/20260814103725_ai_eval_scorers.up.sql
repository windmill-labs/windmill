-- Add up migration script here

-- The scorers a dataset is scored by. One entry per column of the results table:
-- {id, name, kind, ...kind-specific config}. `id` is assigned once and never reused, so a column
-- stays the same column across experiments when it is renamed or its definition is edited — which
-- is what makes a delta between two experiments meaningful.
ALTER TABLE eval_dataset ADD COLUMN scorers JSONB NOT NULL DEFAULT '[]';

-- An experiment is the set of runs over a dataset. The newest one is writable, which is where a
-- single case run lands; running the whole dataset closes it and opens the next.
ALTER TABLE eval_experiment ADD COLUMN label VARCHAR(255) NULL;
ALTER TABLE eval_experiment ADD COLUMN closed_at TIMESTAMPTZ NULL;

-- Experiments that predate the notion of a writable one were each a whole-dataset run, which is
-- exactly what a closed experiment is. The next run opens a fresh one.
UPDATE eval_experiment SET closed_at = created_at WHERE closed_at IS NULL;

-- At most one writable experiment per dataset *and subject*. Two agents evaluated against the same
-- dataset each keep their own working set, so opening evals on one never shows the other's runs,
-- and switching between them does not throw away where you were. Two concurrent case runs on one
-- subject still cannot split the results table in half.
CREATE UNIQUE INDEX index_eval_experiment_open ON eval_experiment
    (workspace_id, dataset_path, (subject ->> 'kind'), (subject ->> 'path'))
    WHERE closed_at IS NULL;

-- Serves the per-subject experiment list, which is what the pane opens with.
CREATE INDEX index_eval_experiment_subject ON eval_experiment
    (workspace_id, dataset_path, (subject ->> 'path'), created_at DESC);

-- Rerunning a case replaces its cell rather than appending a second row for the same case, so the
-- pair is what identifies a cell. Inline (dataset-less) runs never reach this table.
ALTER TABLE eval_experiment_case ADD CONSTRAINT eval_experiment_case_unique_case
    UNIQUE (experiment_id, case_id);

-- The resource version the agent was at for *this* cell. It lives here rather than on the
-- experiment because a writable experiment can hold cells run before and after an edit, and
-- averaging those without saying so is how an eval lies.
ALTER TABLE eval_experiment_case ADD COLUMN subject_version BIGINT NULL;
ALTER TABLE eval_experiment_case ADD COLUMN started_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Which scorers ran is told by the scores themselves, one row per (run, scorer), and which
-- columns exist is the dataset's. A copy on the experiment could only disagree with both.
ALTER TABLE eval_experiment DROP COLUMN scorers;

-- One scorer's verdict on one run. Separate from the run because scoring is separate from running:
-- a frozen experiment can be scored by a scorer added later, from the answers it already stored,
-- without re-running the agent.
CREATE TABLE eval_score (
    experiment_id UUID NOT NULL,
    ordinal INT NOT NULL,
    scorer_id VARCHAR(64) NOT NULL,
    -- NULL while the scoring job is still running, and when it failed.
    score DOUBLE PRECISION NULL,
    reason TEXT NULL,
    -- [{name, passed, detail}], for scorers that report per-assertion results.
    checks JSONB NULL,
    error TEXT NULL,
    -- Hash of the scorer configuration that produced this score, including the script hash or flow
    -- version actually executed. Two scores of the same scorer whose definitions differ are still
    -- compared, but the column says the scorer changed rather than letting it read as a change of
    -- agent.
    definition VARCHAR(64) NOT NULL,
    -- The scoring job, for the kinds that need one. Built-ins are computed in the API.
    job_id UUID NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (experiment_id, ordinal, scorer_id),
    FOREIGN KEY (experiment_id, ordinal) REFERENCES eval_experiment_case (experiment_id, ordinal)
        ON DELETE CASCADE
);

-- Serves the harvest of finished scoring jobs.
CREATE INDEX index_eval_score_pending ON eval_score (job_id) WHERE job_id IS NOT NULL AND score IS NULL;

GRANT ALL ON eval_score TO windmill_user;
GRANT ALL ON eval_score TO windmill_admin;

ALTER TABLE eval_score ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON eval_score FOR ALL TO windmill_admin USING (true);

-- Visibility is the experiment's, which is the dataset's. SELECT only, for the reason given on
-- eval_case: a `FOR ALL ... USING` would let read access to a dataset write its scores.
CREATE POLICY see_parent_experiment ON eval_score FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_experiment e
        WHERE e.id = eval_score.experiment_id
    )
);
