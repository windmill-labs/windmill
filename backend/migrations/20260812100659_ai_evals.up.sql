-- Eval datasets and the cases they hold. Path-addressed like every other Windmill object, so the
-- folder a dataset is named by is what grants access to it.
CREATE TABLE eval_dataset (
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    summary VARCHAR(1000) NULL,
    -- The scorers a dataset is scored by. One entry per column of the results table:
    -- {id, name, kind, ...kind-specific config}. `id` is assigned once and never reused, so a
    -- column stays the same column across experiments when it is renamed or its definition is
    -- edited — which is what makes a delta between two experiments meaningful.
    scorers JSONB NOT NULL DEFAULT '[]',
    extra_perms JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_by VARCHAR(50) NOT NULL,
    PRIMARY KEY (workspace_id, path),
    FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

-- A case is the input half of one evaluation: what the agent is fed, and what it was expected to
-- answer. The generated output, the trajectory and every scorer's return value are the job's, not
-- this table's.
--
-- ON UPDATE CASCADE so renaming a dataset carries its cases instead of stranding them.
CREATE TABLE eval_case (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL,
    dataset_path VARCHAR(255) NOT NULL,
    -- {user_message, user_attachments}
    input JSONB NOT NULL DEFAULT '{}',
    expected JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    FOREIGN KEY (workspace_id, dataset_path) REFERENCES eval_dataset (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

-- Serves the paginated case list, which is ordered oldest-first so a case keeps its position as
-- the dataset grows.
CREATE INDEX index_eval_case_dataset ON eval_case (workspace_id, dataset_path, created_at, id);

GRANT ALL ON eval_dataset TO windmill_user;
GRANT ALL ON eval_dataset TO windmill_admin;
GRANT ALL ON eval_case TO windmill_user;
GRANT ALL ON eval_case TO windmill_admin;

ALTER TABLE eval_dataset ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_case ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON eval_dataset FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_case FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON eval_dataset FOR SELECT TO windmill_user
USING (SPLIT_PART(eval_dataset.path, '/', 1) = 'f' AND SPLIT_PART(eval_dataset.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_read'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON eval_dataset FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(eval_dataset.path, '/', 1) = 'f' AND SPLIT_PART(eval_dataset.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON eval_dataset FOR UPDATE TO windmill_user
USING (SPLIT_PART(eval_dataset.path, '/', 1) = 'f' AND SPLIT_PART(eval_dataset.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON eval_dataset FOR DELETE TO windmill_user
USING (SPLIT_PART(eval_dataset.path, '/', 1) = 'f' AND SPLIT_PART(eval_dataset.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));

CREATE POLICY see_own ON eval_dataset FOR ALL TO windmill_user
USING (SPLIT_PART(eval_dataset.path, '/', 1) = 'u' AND SPLIT_PART(eval_dataset.path, '/', 2) = (select current_setting('session.user')));
CREATE POLICY see_member ON eval_dataset FOR ALL TO windmill_user
USING (SPLIT_PART(eval_dataset.path, '/', 1) = 'g' AND SPLIT_PART(eval_dataset.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.groups'), ','))::text[]));

CREATE POLICY see_extra_perms_user_select ON eval_dataset FOR SELECT TO windmill_user
USING (extra_perms ? (select concat('u/', current_setting('session.user'))));
CREATE POLICY see_extra_perms_user_insert ON eval_dataset FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_update ON eval_dataset FOR UPDATE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_delete ON eval_dataset FOR DELETE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);

CREATE POLICY see_extra_perms_groups_select ON eval_dataset FOR SELECT TO windmill_user
USING (extra_perms ?| (select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[]);
CREATE POLICY see_extra_perms_groups_insert ON eval_dataset FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON eval_dataset FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON eval_dataset FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));

-- Whether the session may *write* the dataset at (_workspace_id, _path): the same disjunction the
-- dataset's own write policies use, in one place so the cases that hang off a dataset are governed
-- by exactly the rule the dataset is. A read grant is not enough — writing a case is writing the
-- dataset's contents — so this checks write, not merely visibility.
CREATE OR REPLACE FUNCTION eval_dataset_writable(_workspace_id varchar, _path varchar)
    RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM eval_dataset d
        WHERE d.workspace_id = _workspace_id AND d.path = _path
        AND (
            (SPLIT_PART(d.path, '/', 1) = 'f' AND SPLIT_PART(d.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]))
            OR (SPLIT_PART(d.path, '/', 1) = 'u' AND SPLIT_PART(d.path, '/', 2) = (select current_setting('session.user')))
            OR (SPLIT_PART(d.path, '/', 1) = 'g' AND SPLIT_PART(d.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.groups'), ','))::text[]))
            OR ((d.extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean)
            OR EXISTS (
                SELECT 1 FROM jsonb_each_text(d.extra_perms) ep
                WHERE SPLIT_PART(ep.key, '/', 1) = 'g'
                AND ep.key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
                AND ep.value::boolean)
        )
    );
$$;

-- Cases are the *contents* of a dataset, not independently addressable objects, so both their
-- visibility and who may change them are the parent's, stated once here instead of mirrored in the
-- API and left to drift. Read is the dataset's read (the subquery is itself subject to
-- eval_dataset's SELECT policies above); write is the dataset's write, which `eval_dataset_writable`
-- checks — so a read-only grant on a dataset can list its cases but not edit them. The whole edit
-- of a dataset and its cases therefore runs as one `user_db` transaction, governed by these
-- policies, rather than being split across the unrestricted pool after a hand-written check.
CREATE POLICY see_parent_dataset ON eval_case FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_dataset d
        WHERE d.workspace_id = eval_case.workspace_id AND d.path = eval_case.dataset_path
    )
);
CREATE POLICY write_parent_dataset_insert ON eval_case FOR INSERT TO windmill_user
WITH CHECK (eval_dataset_writable(eval_case.workspace_id, eval_case.dataset_path));
CREATE POLICY write_parent_dataset_update ON eval_case FOR UPDATE TO windmill_user
USING (eval_dataset_writable(eval_case.workspace_id, eval_case.dataset_path))
WITH CHECK (eval_dataset_writable(eval_case.workspace_id, eval_case.dataset_path));
CREATE POLICY write_parent_dataset_delete ON eval_case FOR DELETE TO windmill_user
USING (eval_dataset_writable(eval_case.workspace_id, eval_case.dataset_path));
-- One run of a dataset: written once when the dataset is run, and only ever read afterwards,
-- which is what makes it worth comparing against.
CREATE TABLE eval_experiment (
    id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    dataset_path VARCHAR(255) NOT NULL,
    -- {kind, path, version}: what was run, at the version it was at when the run was enqueued.
    subject JSONB NOT NULL,
    -- A run is named by the number it is: "Run 7" is stable, sorts, and survives history being
    -- pruned, which a position computed at read time would not. Allocated per (dataset, subject
    -- path) when the run is opened.
    run_number INTEGER NOT NULL,
    -- A run is one flow: a loop over the cases, each iteration answering and then scoring. This
    -- is the job holding it, so the run can be watched, cancelled and rerun as the single thing
    -- it is. Assigned before the flow is pushed, so a launch that dies partway leaves an
    -- experiment naming a job that never started rather than a flow nothing accounts for.
    run_job_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    FOREIGN KEY (workspace_id, dataset_path) REFERENCES eval_dataset (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_eval_experiment_dataset ON eval_experiment (workspace_id, dataset_path, created_at DESC);

-- Serves the per-agent run list, which spans datasets: an agent's history is one list whichever
-- dataset each run was of.
CREATE INDEX index_eval_experiment_subject ON eval_experiment
    (workspace_id, (subject ->> 'path'), created_at DESC);

-- The exact case set an experiment ran, by value: a dataset keeps changing, and a result set that
-- cannot say which inputs produced it is not reproducible. `case_id` is therefore deliberately not
-- a foreign key — deleting a case must not rewrite the history of the runs that used it.
CREATE TABLE eval_experiment_case (
    experiment_id UUID NOT NULL REFERENCES eval_experiment (id) ON DELETE CASCADE,
    ordinal INT NOT NULL,
    case_id UUID NOT NULL,
    input JSONB NOT NULL DEFAULT '{}',
    expected JSONB NULL,
    -- The iteration of the run's flow that answered this case. Minted by the flow engine, so the
    -- case is recorded before it has one and the id is filled in once the iterations exist.
    job_id UUID NULL,
    -- What the run produced, copied out of the jobs once they have produced it. Jobs have their
    -- own retention, and a recorded run has to still read as the run it was once they are gone.
    -- `answered` is the agent step's own outcome, which is settled while the iteration around it
    -- is still scoring; `status` is the iteration's, once it has one.
    output TEXT NULL,
    answered BOOLEAN NULL,
    status VARCHAR(30) NULL,
    -- The resource version the agent was at for this cell, and — for a draft, which has no
    -- version to move — the hash of the configuration that actually ran: the only thing that can
    -- say a row describes an agent that has since been edited.
    subject_version BIGINT NULL,
    subject_draft_hash VARCHAR(64) NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (experiment_id, ordinal),
    -- A run holds each case once: the pair is what identifies a cell.
    CONSTRAINT eval_experiment_case_unique_case UNIQUE (experiment_id, case_id)
);

-- One scorer's verdict on one run. Separate from the run because scoring is separate from running:
-- a scorer's verdict is stored per run and scorer, independent of the agent execution that
-- produced the answers.
CREATE TABLE eval_score (
    experiment_id UUID NOT NULL,
    ordinal INT NOT NULL,
    scorer_id VARCHAR(64) NOT NULL,
    -- NULL until the verdict has been read out of the run's flow, and when scoring failed.
    score DOUBLE PRECISION NULL,
    reason TEXT NULL,
    -- [{name, passed, detail}], for scorers that report per-assertion results.
    checks JSONB NULL,
    error TEXT NULL,
    -- The scorer read the run and said it had nothing to measure on this case. A verdict, not a
    -- failure: the cell is left out of the column's mean and pass rate rather than counted as a
    -- zero or reported as a scorer that produced nothing.
    not_applicable BOOLEAN NOT NULL DEFAULT false,
    -- Hash of the scorer configuration that produced this score, including the script hash or flow
    -- version actually executed. Two scores of the same scorer whose definitions differ are still
    -- compared, but the column says the scorer changed rather than letting it read as a change of
    -- agent.
    definition VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (experiment_id, ordinal, scorer_id),
    FOREIGN KEY (experiment_id, ordinal) REFERENCES eval_experiment_case (experiment_id, ordinal)
        ON DELETE CASCADE
);

GRANT ALL ON eval_experiment TO windmill_user;
GRANT ALL ON eval_experiment TO windmill_admin;
GRANT ALL ON eval_experiment_case TO windmill_user;
GRANT ALL ON eval_experiment_case TO windmill_admin;
GRANT ALL ON eval_score TO windmill_user;
GRANT ALL ON eval_score TO windmill_admin;

ALTER TABLE eval_experiment ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_experiment_case ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_score ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON eval_experiment FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_experiment_case FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_score FOR ALL TO windmill_admin USING (true);

-- Experiments are the *contents* of a dataset, not independently addressable objects, so their
-- visibility is the parent's: the subquery is itself subject to eval_dataset's policies, which
-- stay stated once instead of being mirrored here and left to drift.
--
-- SELECT only, deliberately. A `FOR ALL ... USING` would be reused as the INSERT/UPDATE/DELETE
-- check expression, and since the subquery is a SELECT it applies the dataset's *read* policies —
-- which would let someone with read-only access to a dataset forge an experiment row naming a job
-- they cannot otherwise read. Writes are done on the unrestricted pool after the API has checked
-- write access to the parent, and a stray `user_db` write to these tables is meant to fail rather
-- than silently succeed.
CREATE POLICY see_parent_dataset ON eval_experiment FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_dataset d
        WHERE d.workspace_id = eval_experiment.workspace_id AND d.path = eval_experiment.dataset_path
    )
);

CREATE POLICY see_parent_experiment ON eval_experiment_case FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_experiment e
        WHERE e.id = eval_experiment_case.experiment_id
    )
);

-- Visibility is the experiment's, which is the dataset's.
CREATE POLICY see_parent_experiment ON eval_score FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_experiment e
        WHERE e.id = eval_score.experiment_id
    )
);
