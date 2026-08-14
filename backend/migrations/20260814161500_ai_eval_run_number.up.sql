-- An experiment is named by the run it is: "Run 7" is stable, sorts, and survives history being
-- pruned, which a position computed at read time would not.
ALTER TABLE eval_experiment ADD COLUMN run_number INTEGER NOT NULL DEFAULT 0;

WITH numbered AS (
    SELECT id, row_number() OVER (
        PARTITION BY workspace_id, dataset_path, subject ->> 'kind', subject ->> 'path'
        ORDER BY created_at
    ) AS n
    FROM eval_experiment
)
UPDATE eval_experiment e SET run_number = numbered.n
FROM numbered WHERE numbered.id = e.id;
