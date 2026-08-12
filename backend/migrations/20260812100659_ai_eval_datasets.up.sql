-- Add up migration script here

-- Eval datasets and the cases they hold. Path-addressed like every other Windmill object, so the
-- folder a dataset is named by is what grants access to it.
CREATE TABLE eval_dataset (
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    summary VARCHAR(1000) NULL,
    description TEXT NULL,
    -- {kind, path, version}: the subject the drawer offers when the dataset is opened without one.
    default_subject JSONB NULL,
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
    name VARCHAR(255) NULL,
    -- {user_message, user_attachments, messages}
    input JSONB NOT NULL DEFAULT '{}',
    host_flow_path VARCHAR(255) NULL,
    tool_inputs JSONB NULL,
    expected JSONB NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    -- Where the case was captured from, when it came from real traffic rather than being typed.
    source JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    FOREIGN KEY (workspace_id, dataset_path) REFERENCES eval_dataset (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

-- Serves the paginated case list, which is ordered oldest-first so a case keeps its position as
-- the dataset grows.
CREATE INDEX index_eval_case_dataset ON eval_case (workspace_id, dataset_path, created_at, id);

CREATE TABLE eval_experiment (
    id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    dataset_path VARCHAR(255) NOT NULL,
    -- {kind, path, version}: what was run, at the version it was at when the run was enqueued.
    subject JSONB NOT NULL,
    scorers JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    FOREIGN KEY (workspace_id, dataset_path) REFERENCES eval_dataset (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_eval_experiment_dataset ON eval_experiment (workspace_id, dataset_path, created_at DESC);

-- The exact case set an experiment ran, by value: a dataset keeps changing, and a result set that
-- cannot say which inputs produced it is not reproducible. `case_id` is therefore deliberately not
-- a foreign key — deleting a case must not rewrite the history of the runs that used it.
CREATE TABLE eval_experiment_case (
    experiment_id UUID NOT NULL REFERENCES eval_experiment (id) ON DELETE CASCADE,
    ordinal INT NOT NULL,
    case_id UUID NOT NULL,
    name VARCHAR(255) NULL,
    input JSONB NOT NULL DEFAULT '{}',
    expected JSONB NULL,
    -- Assigned before the job is pushed, so a launch that dies partway leaves a recorded case
    -- whose job is missing, rather than a running job no experiment accounts for.
    job_id UUID NOT NULL,
    PRIMARY KEY (experiment_id, ordinal)
);

GRANT ALL ON eval_dataset TO windmill_user;
GRANT ALL ON eval_dataset TO windmill_admin;
GRANT ALL ON eval_case TO windmill_user;
GRANT ALL ON eval_case TO windmill_admin;
GRANT ALL ON eval_experiment TO windmill_user;
GRANT ALL ON eval_experiment TO windmill_admin;
GRANT ALL ON eval_experiment_case TO windmill_user;
GRANT ALL ON eval_experiment_case TO windmill_admin;

ALTER TABLE eval_dataset ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_case ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_experiment ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_experiment_case ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON eval_dataset FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_case FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_experiment FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON eval_experiment_case FOR ALL TO windmill_admin USING (true);

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

-- Cases and experiments are the *contents* of a dataset, not independently addressable objects, so
-- their visibility is the parent's: the subquery is itself subject to eval_dataset's policies
-- above, which stay stated once instead of being mirrored here and left to drift.
--
-- SELECT only, deliberately. A `FOR ALL ... USING` would be reused as the INSERT/UPDATE/DELETE
-- check expression, and since the subquery is a SELECT it applies the dataset's *read* policies —
-- which would let someone with read-only access to a dataset write its cases, and forge an
-- experiment row naming a job they cannot otherwise read. Writes are done on the unrestricted pool
-- after the API has checked write access to the parent, and a stray `user_db` write to these
-- tables is meant to fail rather than silently succeed.
CREATE POLICY see_parent_dataset ON eval_case FOR SELECT TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM eval_dataset d
        WHERE d.workspace_id = eval_case.workspace_id AND d.path = eval_case.dataset_path
    )
);

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
