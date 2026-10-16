This crate is compiled separately from the backend workspace because the bundled DuckDB's libc++
clashes with `deno_core`'s when both live in one process.

The symptom was opaque errors. DuckDB errors worked, except when attached to a DuckLake with the
`deno_core` feature on:

```sql
ATTACH 'ducklake' AS dl; USE dl;
CREATE TABLE IF NOT EXISTS t (x string not null);
INSERT INTO t VALUES (NULL);
```

gave `Unknown exception in ExecutorTask::Execute` instead of `Constraint Error: NOT NULL
constraint failed: t.x`. The opaque error comes from the C++ DuckDB library:
https://github.com/duckdb/duckdb/blob/f99fed1e0b16a842573f9dad529f6c170a004f6e/src/parallel/executor_task.cpp#L58

The worker never links this crate: it runs `windmill_duckdb_runner` (`src/bin`) as a separate
process for every job. See `docs/duckdb-execution.md`.
