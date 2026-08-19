-- A run is one flow: a loop over the cases, each iteration answering and then scoring. The job
-- holding it, so the run can be watched, cancelled and rerun as the single thing it is.
ALTER TABLE eval_experiment ADD COLUMN run_job_id UUID NULL;

-- The iteration's job id is minted by the flow engine, so a case is now recorded before it has
-- one and the id is filled in once the iterations exist.
ALTER TABLE eval_experiment_case ALTER COLUMN job_id DROP NOT NULL;
