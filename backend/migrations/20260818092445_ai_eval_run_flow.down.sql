DELETE FROM eval_experiment_case WHERE job_id IS NULL;
ALTER TABLE eval_experiment_case ALTER COLUMN job_id SET NOT NULL;
ALTER TABLE eval_experiment DROP COLUMN run_job_id;
