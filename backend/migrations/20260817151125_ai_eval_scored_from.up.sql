-- A run whose answers came from another run, measured again by the scorers as they are now.
-- Scoring is separate from running, so a scorer edited or added after a run should be able to
-- measure what that run already answered — and a run is permanent, so it cannot be measured in
-- place. The answers and their version stamp are copied whole, which is what keeps this different
-- from a run of mixed provenance: every cell of it was produced by the version recorded here.
ALTER TABLE eval_experiment ADD COLUMN scored_from UUID NULL;
