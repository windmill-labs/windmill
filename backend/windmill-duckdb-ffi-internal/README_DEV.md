This crate is compiled separately because it causes nasty issues when compiled with the deno_core feature flag enabled (lib c++ interactions).

The main issue was :
Errors in DuckDB always worked correctly, except when attached to a Ducklake and when the deno_core feature flag was set.
For example:

```sql
ATTACH 'ducklake' AS dl; USE dl;
CREATE TABLE IF NOT EXISTS t (x string not null);
INSERT INTO t VALUES (NULL);
```

causes `Constraint Error: NOT NULL constraint failed: t.x` normally, but here we see `Unknown exception in ExecutorTask::Execute`. This opaque errors comes directly from the C++ DuckDB library : https://github.com/duckdb/duckdb/blob/f99fed1e0b16a842573f9dad529f6c170a004f6e/src/parallel/executor_task.cpp#L58

To solve this, we compile duckdb separately from the main backend crate and call it with FFI. It has to be loaded dynamically, if it is loaded statically it will still share lib c++ with deno_core and cause issues.
