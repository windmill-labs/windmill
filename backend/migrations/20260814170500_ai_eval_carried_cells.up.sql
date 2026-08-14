-- A run started to rerun one case carries the rest of the previous run's results, so a partial
-- rerun is a comparable run rather than a table with one number in it. The column says which
-- cells were carried, which is what tells the two apart afterwards.
ALTER TABLE eval_experiment_case ADD COLUMN carried_from UUID NULL;

-- A run seeded from another one is provisional until it is kept: the alert offering that is
-- driven by this, not by whether the run happens to be the writable one.
ALTER TABLE eval_experiment ADD COLUMN seeded_from UUID NULL;
