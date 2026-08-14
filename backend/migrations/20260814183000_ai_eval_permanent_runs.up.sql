-- A run is permanent. Rerunning one case opens a run of its own, seeded from the one before it,
-- so nothing is ever written over: there is no writable experiment left to keep or discard, and
-- the index that enforced one of them per subject has nothing to enforce.
DROP INDEX IF EXISTS index_eval_experiment_open;
ALTER TABLE eval_experiment DROP COLUMN closed_at;

-- What a draft run actually ran. A draft has no version to move, so a hash of the configuration
-- is the only thing that can say a row describes an agent that has since been edited.
ALTER TABLE eval_experiment_case ADD COLUMN subject_draft_hash VARCHAR(64) NULL;
