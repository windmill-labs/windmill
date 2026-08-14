-- Add down migration script here
DROP TABLE IF EXISTS eval_score;
ALTER TABLE eval_experiment ADD COLUMN IF NOT EXISTS scorers JSONB NOT NULL DEFAULT '[]';
DROP INDEX IF EXISTS index_eval_experiment_open;
DROP INDEX IF EXISTS index_eval_experiment_subject;
ALTER TABLE eval_experiment_case DROP CONSTRAINT IF EXISTS eval_experiment_case_unique_case;
ALTER TABLE eval_experiment_case DROP COLUMN IF EXISTS subject_version;
ALTER TABLE eval_experiment_case DROP COLUMN IF EXISTS started_at;
ALTER TABLE eval_experiment DROP COLUMN IF EXISTS closed_at;
ALTER TABLE eval_experiment DROP COLUMN IF EXISTS label;
ALTER TABLE eval_dataset DROP COLUMN IF EXISTS scorers;
