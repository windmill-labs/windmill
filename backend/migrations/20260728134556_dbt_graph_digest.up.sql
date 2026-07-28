-- A digest of the graph a row set describes, so a run can tell whether its
-- graph differs from the version's before storing a copy of it.
--
-- `graph_is_per_run` is set whenever `vars` holds a `{{ }}` placeholder or `env`
-- holds a `$var:` — a conservative trigger, not evidence the model set varies.
-- The common dynamic descriptor is a date var, which changes the data and not
-- which models exist, so its every run produced a byte-identical snapshot.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS graph_digest TEXT;
