# DuckLake-native materialization

Design sketch for "managed, versioned, incremental" assets built on the
DuckLake substrate. This is a companion to [`pipelines-vs-dbt.md`](./pipelines-vs-dbt.md)
and extends its **Path C (hybrid, partition-first)** recommendation. The new
contribution here is leveraging DuckLake's snapshot/time-travel layer, which
the earlier doc's incremental deep-dive did not use. The annotation grammar is
reconciled with that doc — `// partitioned` + `// unique_key` + `// append`
stay canonical; nothing here forks a competing vocabulary.

## The core reframe

dbt had to *build* a materialization engine (compile SQL → `CREATE TABLE AS` /
incremental `MERGE` / SCD2 snapshots) because the warehouse gives it nothing
but raw SQL over a mutable table. DuckLake hands us, at the storage layer, the
four things that engine exists to provide:

| Capability | Source |
|---|---|
| ACID multi-statement transactions | DuckLake |
| A snapshot per commit + time-travel (`AT (VERSION => n)` / `AT (TIMESTAMP => ...)`) | DuckLake |
| Physical partitioning + pruning (`ALTER TABLE … SET PARTITIONED BY (…)`) | DuckLake |
| Schema evolution tracked in the catalog DB | DuckLake |

Windmill already attaches DuckLake fully — see `transform_attach_ducklake`
(`backend/windmill-worker/src/duckdb_executor.rs:661`), which rewrites a user's
`ATTACH 'ducklake://name' AS dl` into the real
`ATTACH 'ducklake:postgres:…' AS dl (DATA_PATH 's3://…', OVERRIDE_DATA_PATH
TRUE, AUTOMATIC_MIGRATION TRUE)` at `duckdb_executor.rs:730`. But today every
write is a destructive overwrite and **all four capabilities above are thrown
away** — snapshots are never surfaced, partitioning is purely an orchestration
concept disconnected from the physical layout.

So the materialization engine we need is a thin layer — a write-strategy
wrapper + snapshot capture — not a dbt rebuild. This is exactly the
"buy A's 80% without B's dialect-rewriting tax" tradeoff `pipelines-vs-dbt.md`
argued for; DuckLake is what makes the remaining 20% (versioning, reproducible
reads, materialization history) nearly free instead of a second project.

## Annotation grammar (final)

One self-documenting line; managed-by-default. Strategy options live *on* the
`materialize` line (they have no meaning without it), while `// partitioned`
stays separate because it is cross-cutting (cascade + scheduling + materialize).

```
// materialize ducklake://analytics/orders_daily              → managed, replace (default)
// materialize ducklake://analytics/orders_daily key=order_id → managed, merge
// materialize ducklake://analytics/orders_daily append       → managed, append
// materialize manual ducklake://analytics/orders_daily       → track-only escape hatch
```

- **managed (default)** — the script is *setup + one trailing `SELECT`*; Windmill
  generates the write DDL, captures the DuckLake snapshot, and records state.
  DuckDB-only; validated at deploy (a non-SELECT script is rejected with a clear
  error pointing to the `wmll.ducklake` helpers).
- **`manual`** — escape hatch: the script writes its own DDL; Windmill only
  records state (no snapshot capture, no idempotency guarantee). Rare; explicit.
- **`key=<col>`** → MERGE (dedup within slice); **`append`** → INSERT-only;
  neither → DELETE-by-partition + INSERT (replace). `append` wins over `key` if
  both are given (deploy warning).
- **`// partitioned <kind>`** — unit of work + state + backfill (separate;
  cross-cutting). Polyglot / multi-statement writes use the `wmll.ducklake`
  helpers instead of `// materialize`.

There is no `wrap` keyword — `materialize` *is* "manage the write," so it was
redundant; the only reason for it was to carve out the weak track-only mode,
which is now the explicit `manual` opt-out.

DuckLake snapshots are **orthogonal to all of the above** — they apply to every
strategy automatically because every write is a DuckLake commit. The user never
annotates for versioning; they get it.

## Executor codegen

### The seam

`run_duckdb` already splits the script into statement blocks and rewrites
custom `ATTACH` blocks in a single pass before execution
(`duckdb_executor.rs:114-160`):

```rust
let query_block_list = parse_sql_blocks(&query, true);
// each block: remove_comments → if ducklake/datatable ATTACH, expand; else passthrough
```

All blocks run in order on one DuckDB connection. Two integration points:

1. **Literal mode (default).** The user writes their own DELETE+INSERT / MERGE
   inside their own `BEGIN … COMMIT` (the template scaffolds it — see Path A
   example in `pipelines-vs-dbt.md`). Windmill injects nothing into the body;
   it only (a) binds `WM_PARTITION_*` context and (b) appends a snapshot-capture
   block (below). DuckLake makes the user's `BEGIN…COMMIT` atomic for free.

2. **Wrap mode (`// materialize wrap`).** The user writes one `SELECT`. Windmill
   replaces that block with generated statements, wrapped in an *explicit
   DuckLake transaction it controls* — never textual `BEGIN/COMMIT` injected
   around the user's other statements (fragile across their own `ATTACH`s and
   multi-statement SQL). Concretely, the single SELECT block expands to:

   ```sql
   -- generated for: // partitioned daily ; target = dl.orders_daily
   BEGIN TRANSACTION;
   CREATE TABLE IF NOT EXISTS dl.orders_daily AS <user_select> WHERE false;  -- first-run bootstrap, schema only
   DELETE FROM dl.orders_daily WHERE _wm_partition = $WM_PARTITION;
   INSERT INTO dl.orders_daily
     SELECT *, $WM_PARTITION AS _wm_partition FROM (<user_select>);
   COMMIT;
   ```

   With `// unique_key order_id`, the DELETE+INSERT becomes a `MERGE INTO
   dl.orders_daily USING (<user_select>) ON order_id` (dedup within slice).
   With `// append`, the DELETE is dropped. This is the *only* place dialect
   templating lives, and it's DuckDB-only — we never pay dbt's cross-warehouse
   MERGE tax.

The `_wm_partition` column is the physical link the current orchestration layer
lacks: on first materialize, Windmill also runs `ALTER TABLE dl.orders_daily
SET PARTITIONED BY (_wm_partition)` so DuckLake prunes on read and the
DELETE-by-partition rewrites only that partition's Parquet files.

### Snapshot capture

After the user's (or generated) blocks, append one read block and record the
result onto the run:

```sql
SELECT max(snapshot_id) AS snapshot_id FROM ducklake_snapshots('dl');
```

The returned `snapshot_id` (plus `INSERT`/`MERGE` row count, already available
from DuckDB's result) is persisted as materialization metadata. This is a
single extra round-trip per materialization, no new infra.

## Metadata schema

Extends the `materialized_partitions` table proposed in `pipelines-vs-dbt.md`
§"First implementation slice" with the DuckLake snapshot id:

```
materialized_partition (
  workspace_id      TEXT,
  asset_kind        TEXT,        -- 'ducklake'
  asset_path        TEXT,        -- 'analytics/orders_daily'
  partition         TEXT,        -- '2026-06-19'  (NULL for unpartitioned)
  snapshot_id       BIGINT,      -- DuckLake snapshot produced by this materialize
  row_count         BIGINT,
  job_id            UUID,
  materialized_at   TIMESTAMPTZ,
  PRIMARY KEY (workspace_id, asset_kind, asset_path, partition)
)
```

This one table drives four things at once:

- **Observability** — "last materialized: snapshot 42, 1.2M rows, 09:14" per
  asset node (closes the Dagster-catalog gap from the v1-readiness review).
- **Run-stale / gap detection** — which partitions exist, which are missing.
- **Backfill** — the missing/failed set *is* the backfill worklist.
- **Snapshot pinning** — see below.

## Reproducibility — the beyond-dbt part

Because every materialization records the snapshot it produced, a downstream
consumer can read the *exact* upstream snapshot its run saw:

```sql
FROM dl.orders_daily AT (VERSION => $WM_UPSTREAM_SNAPSHOT)
```

The cascade already threads a `trigger` blob (producer path, partition) to each
subscriber; add the producer's captured `snapshot_id` to it, and a consumer's
read is pinned to the upstream state at dispatch time. That makes the *whole
pipeline* reproducible and time-travelable — something dbt has no native answer
for (dbt models are always "whatever's in the warehouse now"). It also gives
rollback (re-point an asset to snapshot N) and "what did this table look like at
the failing run" debugging, for free off the same captured ids.

This is the differentiator worth leaning on. It is not catch-up to dbt; it is a
capability dbt structurally cannot offer, and DuckLake gives it to us at the
cost of recording one integer per run.

It also means **we do not build SCD2 snapshots** (gap #4 in `pipelines-vs-dbt.md`):
DuckLake time-travel is a strictly better answer for most of what dbt's
`{% snapshot %}` is used for. One fewer engine to write.

## Scoping decision: DuckLake vs DataTable

**Make DuckLake the materialization/versioning substrate; keep DataTable as the
live operational table with no versioning.** DataTable is plain Postgres
(`transform_attach_datatable`, `duckdb_executor.rs:742`) — no native snapshots
or time-travel — so giving *it* the versioned/incremental story means building
MVCC-on-top ourselves (history tables, SCD2), precisely the complexity this
DuckLake approach exists to avoid. Clean split:

- `ducklake://` → analytics, versioned, reproducible, backfillable.
- `datatable://` → mutable app/operational state; partition idempotency via
  DELETE+INSERT still works, but no snapshot/time-travel layer.

Don't try to give both the full treatment for v1.

## v1 slice (smallest viable)

1. **Partition runtime context** — resolve `(value, start, end)` and surface as
   `WM_PARTITION*` bind/env (Path C step 1; partly built per the
   pipeline-partition-runtime work).
2. **Physical partition wiring** — `_wm_partition` column + `SET PARTITIONED BY`
   on first materialize for `ducklake://` targets.
3. **Strategy templates** — DELETE+INSERT default; MERGE when `// unique_key`;
   INSERT when `// append`. Literal scaffolds first; `// materialize wrap` for
   single-SELECT DuckDB behind the same templates.
4. **Snapshot + metadata capture** — append `ducklake_snapshots` read, persist
   `materialized_partition` rows.
5. **Surface it** — last-materialized/snapshot/row-count on the asset node;
   missing-partition set feeds the backfill UI.
6. *v1.x* — snapshot pinning across the cascade (`$WM_UPSTREAM_SNAPSHOT`),
   rollback, time-travel read helper.

Steps 1–5 are a thin annotation+template layer plus one metadata table and one
extra read per run. They deliver managed/incremental/versioned assets,
idempotent partitioned materialization, the backfill substrate, and
materialization observability together — and stay recognizably Windmill-shaped.

## Open decisions

These ride on top of the six in `pipelines-vs-dbt.md` §"Decisions either path
forces"; DuckLake-specific:

1. **Bootstrap of `SET PARTITIONED BY`.** First-materialize detection — table
   absent vs. present-but-unpartitioned. Idempotent re-apply.
2. **Snapshot retention / compaction.** DuckLake snapshots accumulate; when do
   we expire old ones, and does pinning hold a snapshot alive past retention?
3. **Pin scope.** Pin only direct producers, or the full transitive upstream
   set per run? Storage and "stale pin" semantics differ.
4. **Wrap-mode multi-statement.** `// materialize wrap` assumes one trailing
   SELECT; define behavior (reject? last-SELECT-wins?) when the script has
   setup statements before it.
