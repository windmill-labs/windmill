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
// materialize ducklake://analytics/orders_daily key=order_id → managed, merge (SCD type 1)
// materialize ducklake://analytics/orders_daily append       → managed, append
// materialize ducklake://analytics/dim_customer key=id history → managed, SCD type 2 history
// materialize manual ducklake://analytics/orders_daily       → track-only escape hatch
```

- **managed (default)** — the script is *setup + one trailing `SELECT`*; Windmill
  generates the write DDL, captures the DuckLake snapshot, and records state.
  DuckDB-only; validated at deploy (a non-SELECT script is rejected with a clear
  error pointing to the `wmll.ducklake` helpers).
- **`manual`** — escape hatch: the script writes its own DDL; Windmill only
  records state (no snapshot capture, no idempotency guarantee). Rare; explicit.
- **`key=<col>`** → MERGE (dedup within slice, SCD type 1 — overwrites history);
  **`append`** → INSERT-only; neither → DELETE-by-partition + INSERT (replace).
  `append` wins over `key` if both are given (deploy warning).
- **`key=<col> history [track=<c1,c2,…>] [deletes=close]`** → upgrades the keyed
  merge to managed SCD **type 2** history (the leading keyword `scd2` is an alias).
  The SELECT is the *current snapshot* (one row per `key`), and the runtime adds
  `valid_from`/`valid_to`/`is_current`; a change to any tracked column (`track=`,
  default all non-key) closes the prior version and opens a new one, keeping full
  history. Diff → close-old (`UPDATE`) → open-new (`INSERT`) in one transaction;
  the effective timestamp is the transaction clock (`now()`), so a run is
  self-consistent. v1 is **non-partitioned only** (`// partitioned` + history is
  rejected). Unlike `manual`, it is managed, so `// data_test` and schema capture
  work.
  - **Deletes.** By default a key that disappears from the snapshot stays current
    (soft delete — dbt's `hard_deletes=ignore`). `deletes=close` opts into
    hard-delete-close: the vanished key's current version is closed (`valid_to`
    set, `is_current=false`) with no new version — dbt's `hard_deletes=close`. If
    that key later reappears in the snapshot it opens a fresh version, leaving a
    validity gap between the delete and the reactivation (correct SCD2).
  - **Reserved names.** `valid_from`/`valid_to`/`is_current` are reserved column
    names in this mode — a SELECT that already projects one fails at run time —
    and the `<dim>_current` suffix is reserved for the companion view (below), so
    don't separately materialize a table by that name in the same lake (the view
    is created `IF NOT EXISTS` inside the write transaction, so such a collision
    is skipped silently — the `_current` convenience is simply absent — rather
    than erroring).
  - **Key should be non-null.** A `NULL` natural key is ill-formed for a
    dimension; the codegen matches keys null-safely so a `NULL`-key row is
    materialized rather than silently dropped, but you should enforce it with
    `// data_test not_null <key>`.
  - **`track=` takes no spaces.** Like every `=`-option in the annotation grammar
    (which is whitespace-tokenized), the `track=` value must be a bare
    comma-separated list with no spaces: `track=name,tier`, not `track=name, tier`
    (a space ends the value and the rest is silently ignored).
  - **Schema is frozen at first run** (persist-and-mutate, like `merge`/`append`):
    the history table is `CREATE TABLE IF NOT EXISTS`, so adding/removing a
    projected column later fails the run — an append-only history can't retroactively
    reshape closed versions. Changing the SELECT's columns needs a manual rebuild
    (the `replace` strategy is the only one that re-derives schema each run).
  - **Consumer convenience.** Each run (re)creates a `<dim>_current` view (`WHERE
    is_current`) in the same catalog, so the common "latest version" read needs no
    filter and downstream scripts can `// on ducklake://…/<dim>_current`. The
    effective-dated payoff is a native DuckDB `ASOF JOIN` against the history
    table: `… ASOF JOIN <dim> d ON fact.key = d.<key> AND fact.ts >= d.valid_from`
    returns, for each fact, the dimension version that was current at `fact.ts` —
    something neither `merge` nor DuckLake time-travel can do in one query.
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

All blocks run in order on one DuckDB connection. Materialize has two modes:

1. **Managed (default).** The user writes *setup + one trailing `SELECT`*. Windmill
   replaces that SELECT with generated statements, wrapped in an *explicit
   DuckLake transaction it controls* — never textual `BEGIN/COMMIT` injected
   around the user's other statements (fragile across their own `ATTACH`s and
   multi-statement SQL). For a partitioned `replace` the SELECT block expands to:

   ```sql
   -- generated for: // partitioned daily ; target = dl.orders_daily ; partition = '2026-06-19'
   CREATE TABLE IF NOT EXISTS dl.orders_daily AS
     SELECT *, CAST(NULL AS VARCHAR) AS _wm_partition FROM (<user_select>) WHERE false;  -- first-run bootstrap
   ALTER TABLE dl.orders_daily SET PARTITIONED BY (_wm_partition);
   BEGIN TRANSACTION;
   DELETE FROM dl.orders_daily WHERE _wm_partition = '2026-06-19';
   INSERT INTO dl.orders_daily SELECT *, '2026-06-19' AS _wm_partition FROM (<user_select>);
   COMMIT;
   ```

   The strategy variants are all DELETE+INSERT-shaped — no `MERGE INTO`, which
   DuckLake can't reliably run on a fresh partition (it 404s writing the first
   rows):
   - **whole-table replace** (no `// partitioned`) → a single `CREATE OR REPLACE
     TABLE … AS <user_select>` (handles schema changes, still snapshots).
   - **`key=<col>`** → `DELETE FROM … WHERE [<partition> AND] <col> IN (SELECT
     <col> FROM (<user_select>))` then `INSERT` (upsert within the slice).
   - **`append`** → the `DELETE` is dropped (insert-only).

2. **`manual`.** The user writes their own DDL inside their own `BEGIN … COMMIT`;
   Windmill injects nothing into the body and only records state (no snapshot
   capture, no idempotency guarantee).

The `_wm_partition` column is the physical link the orchestration layer lacks: on
first materialize Windmill runs `ALTER TABLE … SET PARTITIONED BY (_wm_partition)`
so DuckLake prunes on read and DELETE-by-partition rewrites only that partition's
Parquet files.

> Storage: DuckLake writes go to `s3://_default_/` through the windmill S3 proxy
> (`/api/w/{ws}/s3_proxy`, gated behind the `parquet` + `private` features). The
> proxy must sign the SigV4 canonical URI with **single** percent-encoding — the
> SigV4 default (`Double`) 401s Hive-partition keys like `_wm_partition=2026-06-19`
> (the `=` double-encodes to `%253D` vs the client's `%3D`).

### Run summary capture

After the generated blocks, Windmill appends one read block — it is both the job's
result (a useful preview rendered as the materialized table) and the row it records:

```sql
SELECT 'ducklake://<name>/<table>' AS materialized,
       '<partition>' AS partition,            -- only when partitioned
       (SELECT count(*) FROM <target> [WHERE _wm_partition = '<partition>']) AS rows,
       (SELECT max(snapshot_id) FROM ducklake_snapshots('<target>')) AS snapshot_id;
```

The `snapshot_id` and `rows` are persisted as `materialized_partition` metadata.
One extra round-trip per materialization, no new infra.

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

Because every materialization records the snapshot it produced, you can read any
table *as of* a past version — a capability dbt has no native answer for (dbt
models are always "whatever's in the warehouse now"). DuckLake gives us this for
the cost of recording one integer per run, and it covers the three things people
actually reach for: **debugging** ("what did this table look like at the failing
run"), **rollback** (re-materialize a consumer from snapshot N), and ad-hoc
**experimentation** on a historical state.

### How it's surfaced (shipped): explicit, discoverable time-travel

The version is exposed as a *user-driven* surface, not hidden plumbing. A
consumer pins a read by writing the DuckLake clause directly:

```sql
FROM dl.orders_daily AT (VERSION => 42)
```

The asset node's **History** tab is a master-detail view: the snapshot list (id
+ time) on the left selects the version previewed in a read-only grid on the
right, which surfaces — and copies — the catalog-qualified
`FROM lake.<table> AT (VERSION => n)` clause. Snapshot ids are captured automatically; the user opts
into pinning when they want it, and the clause degrades to "latest" if removed,
so the same script still runs standalone. Mechanically this rides on
time-travel **reads** (`make_select_query` / `make_count_query` emit the `AT`
clause when a `version` is threaded through the `WM_INTERNAL_DB_*` markers) plus
a `DUCKLAKE_SNAPSHOTS` read for the history list — capabilities DuckLake already
has, no new write path.

### Deferred: automatic snapshot pinning across the cascade

An earlier sketch had the cascade *automatically* thread each producer's
`snapshot_id` into the `trigger` blob and inject `AT (VERSION => $WM_UPSTREAM_SNAPSHOT)`
into consumer reads, so a whole run is pinned to upstream state at dispatch time
without anyone asking. This is deliberately **not** built, for three reasons:

- **Not critical.** The only thing it adds over the explicit surface above is
  *automatic per-run consistency* — protection against an upstream
  re-materializing in the window between dispatch and a consumer reading. That
  race only bites high-frequency event-driven cascades (rare today), and the
  read is always a whole, ACID snapshot regardless — never corruption, just
  "newer than the triggering version". Debugging and rollback are already
  covered by the explicit surface.
- **Implicit magic.** Auto-injecting an `AT` clause and stripping it on
  standalone runs is invisible behaviour to debug when it misfires; the explicit
  clause is inspectable.
- **Multi-upstream ambiguity + EE coupling.** A consumer reading two ducklake
  upstreams needs a per-ref snapshot *map* accumulated across the AND-join — and
  the join-slot logic is EE. A single `$WM_UPSTREAM_SNAPSHOT` would silently pin
  every read to one (the firing) producer's snapshot.

If a workload ever shows the consistency race in practice, pinning can be layered
on top — the capture and the snapshot surfacing built here are its foundation.

This is distinct from **SCD2 history** (`// materialize … key=… history`), which *is* built:
DuckLake time-travel answers "what did the whole table look like at snapshot N?"
but not "give me each entity's version history as queryable rows"
(`valid_from`/`valid_to`/`is_current`) — the shape dbt's `{% snapshot %}` produces
and downstream dimensional queries join against. The scd2 strategy generates and
manages that history table (see the strategy bullet above).

## Data tests (`// data_test`) — and the extensible-annotation pattern

Data tests are the first dbt-parity gap closed on top of materialization, and
the **first deliberately extensible annotation**. The design goal was not just
"add five test types" but to establish the convention a sibling family
(column-lineage is the next one) follows, so the annotation vocabulary stops
being a closed hardcoded list (`pipelines-vs-dbt.md` gap #7).

### Grammar

```
// data_test unique <col>
// data_test not_null <col>
// data_test accepted_values <col> = a,b,c
// data_test relationships <col> -> datatable://other/asset.<col>
// data_test <script_path>            ← escape hatch (dbt's singular test)
```

`// data_test` lines **accumulate** (every well-formed line adds one check),
unlike the single-value annotations (`// materialize`, `// partitioned`, …)
which are first-write-wins. Malformed lines are dropped fail-safe — a typo
becomes an *absent* check (visible in the graph), never a mis-parsed one.

The keyword is `data_test`, **not `test`** — there is an unrelated, shipped
`// test:` CI-test annotation (`windmill_common::schema::parse_ci_test_annotation`,
tests a script's *logic* on deploy). `data_test` tests the *data* in the
materialized asset at run time. This mirrors dbt 1.8's own `tests:` →
`data_tests:` rename, made for exactly this disambiguation.

### The pattern: annotation → verifier

The reusable shape, in three layers, each a clean extension seam:

1. **Parse** (`asset_parser.rs` + `parsePipelineAnnotations.ts`, kept in
   lockstep by the parity corpus). A `data_test` line is dispatched on a
   **keyword head** to a typed variant (`DataTest`). A new built-in is one
   match arm + its sub-parser; the `Custom` arm is the open fallback. A sibling
   family reuses this head-keyword dispatch rather than adding a parallel list.
2. **Compile** (`sql_materialize.rs::build_data_test_checks`). Each test becomes
   a **check**: `(name, violating-row-count query)`. Built-ins differ only in
   their count query; `Custom` supplies its own (the user's SELECT of violating
   rows). Referenced assets (relationships) emit an `ATTACH` resolved by the
   same transform pass as the user's own.
3. **Execute** (`duckdb_executor.rs`). The materialize summary query embeds
   every check's count in one `data_tests` list-of-struct column (computed in a
   CTE, since DuckDB rejects subqueries inside struct literals), so **all tests
   run in a single pass** against the freshly-materialized slice — no
   abort-on-first. The worker reads the breakdown from the result and decides
   pass/fail: any violation **fails the run** (record `Failed`, propagate up the
   cascade) with an error listing *every* test (✓/✗ + counts); a clean run
   returns the per-test summary so the UI can render a checklist.

A new annotation family that produces post-materialize checks (or, for
column-lineage, post-materialize *metadata reads*) plugs into the same three
seams: add a parsed variant, emit its check/reader SQL into the summary, read
it back in the worker. Nothing about the closed set of *today's* keywords is
load-bearing.

### Scoping decisions (v1)

- **Partition scope.** When `// partitioned`, built-in checks are scoped to the
  slice just written (`WHERE _wm_partition = <value>`), so a rerun/backfill of
  one partition is independent of other partitions' (possibly pre-existing)
  data. Whole-table assertions are a follow-up.
- **SCD2 scope.** On a `key=… history` target, built-in checks assert the
  *current snapshot* (`WHERE is_current`): the history table legitimately
  repeats the natural key across closed versions, so an unscoped
  `unique(<key>)` would fail the run on the second change of any key. Custom
  tests see the raw history and scope themselves.
- **Commit-then-test (public) / write-audit-publish (enterprise).** In the
  public build, like dbt, the write commits before tests run; a failed test
  fails the *run* (and records `Failed`, so downstream cascade stops) but does
  not roll back the committed snapshot — time-travel still lets you inspect
  exactly what failed. Enterprise upgrades this to write-audit-publish: the
  same checks also run *inside* the write transaction as a guard statement
  (`sql_materialize::data_test_guard_sql`, gated by
  `pipeline_advanced::transactional_data_tests()`) that raises on any
  violation, aborting the run before `COMMIT` — a failing slice is never
  published, readers keep the previous version, and no snapshot is created.
  When guarded, the bootstrap DDL (`CREATE TABLE IF NOT EXISTS`,
  `SET PARTITIONED BY`, the SCD2 history bootstrap) also moves inside the
  transaction, so even the *first* run of a new asset rolls back to "no table"
  rather than leaving an empty shell. The guard is a pure read; on the passing
  path the post-commit summary recomputes the counts for the recorded per-test
  breakdown (probe counts are cheap next to the write).
- **Custom = DuckDB SQL, server worker.** The escape hatch fetches the deployed
  script's content (a single DuckDB `SELECT`/CTE returning the violating rows —
  it's embedded as a subquery, so a multi-statement body is rejected with a
  clear error) and inlines it as a check; `{partition}` is substituted and
  `_wm_target` is in scope. Agent (Http) workers — which have no script cache —
  get a clear error. Non-DuckDB custom tests (dispatched as sub-jobs, any
  language) are the natural follow-up and fit the same verifier seam.
- **Managed only.** `// materialize manual` + `// data_test` is rejected with a
  clear error (we can't know the manual script's target alias / partition col).

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
3. **Strategy templates** — DELETE+INSERT default (`CREATE OR REPLACE` for the
   whole table); delete-by-key + insert when `key=<col>`; INSERT-only when
   `append`. Managed `// materialize` wraps a single-SELECT DuckDB script behind
   these templates; `// materialize manual` opts out.
4. **Snapshot + metadata capture** — append `ducklake_snapshots` read, persist
   `materialized_partition` rows.
5. **Surface it** — last-materialized/snapshot/row-count on the asset node;
   missing-partition set feeds the backfill UI.
6. *v1.x* — time-travel UX over the captured snapshots: a per-asset **History**
   tab — a master-detail snapshot list + query-at-version preview that copies the
   full `FROM lake.<table> AT (VERSION => n)` clause. Automatic cascade pinning
   (`$WM_UPSTREAM_SNAPSHOT`) is deferred — see §"Reproducibility" for why.

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
4. **Managed multi-statement.** *Resolved:* managed `// materialize` accepts
   setup statements (ATTACH/SET/…) followed by exactly one trailing SELECT, and
   rejects anything else at deploy with a clear error pointing to
   `// materialize manual`. The classifier (`sql_materialize.rs`) is the single
   source of truth.
