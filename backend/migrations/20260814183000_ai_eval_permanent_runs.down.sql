ALTER TABLE eval_experiment_case DROP COLUMN subject_draft_hash;
ALTER TABLE eval_experiment ADD COLUMN closed_at TIMESTAMPTZ NULL;
CREATE UNIQUE INDEX index_eval_experiment_open ON eval_experiment
    (workspace_id, dataset_path, (subject ->> 'kind'), (subject ->> 'path'))
    WHERE closed_at IS NULL;
