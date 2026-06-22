# Pipelines vs. dbt

Positioning analysis and architectural notes for the data-pipeline abstraction
currently landing on `feat/asset-graph-view`. Covers what we're building, how
it differs from dbt, which dbt features are real gaps vs. TODO, and a focused
deep-dive on incremental materialization — including a recommendation to
collapse it into partitioning rather than ship it as a separate concept.

## What we're building

Asset-centric, polyglot, annotation-driven, event-aware:

- Assets (`datatable`, `ducklake`, `s3object`, `volume`) are graph nodes;
  scripts are edges that produce/consume them. See
  `backend/parsers/windmill-parser/src/asset_parser.rs:25`.
- Lineage comes from two sources: parsed annotations (`// pipeline`,
  `// on datatable://...`, `// partitioned daily`, `// freshness 1h`,
  `// trigger any`, `// debounce`, `// tag`, `// retry`) and body-inferred
  reads/writes via the asset parser.
- Triggers are first-class: schedule, webhook, email, kafka, mqtt, nats,
  postgres, sqs, gcp — all wired into the same DAG view
  (`frontend/src/lib/components/assets/AssetGraph/types.ts:56`).
- Per-language scaffolds (DuckDB ATTACH, Postgres, Python, TS, Bash) generate
  starter code per `PipelineOutputKind` (`datatable | ducklake | s3_parquet |
  s3_object | none`). See
  `frontend/src/lib/components/assets/AssetGraph/pipelineTemplates.ts:11`.

## Differentiators vs. dbt

1. **Event-driven + batch in one DAG.** dbt is batch-on-warehouse. Kafka →
   Python normalize → DuckDB aggregate → Postgres view → Slack notify is
   native here; in dbt land it's "use Airflow/Prefect for the non-SQL parts."
2. **Polyglot, not SQL+Jinja.** Python/TS/Bash/Duck/PG transformations live
   in the same graph. No Jinja templating language; annotations are real
   comments parsed strictly.
3. **Multi-substrate by design.** `datatable` (Postgres), `ducklake`
   (lakehouse), `s3_parquet`, `s3_object` are peers. dbt's universe is
   "tables in your warehouse."
4. **One platform.** Same runtime as workflows, internal apps, background
   jobs, RBAC, secrets, schedules. dbt is single-purpose.
5. **Inferred lineage from code.** Body parser picks up `CREATE TABLE` / S3
   writes — annotations are not strictly required to get edges. dbt requires
   explicit `{{ ref() }}` everywhere.

## Where dbt wins today

| Gap | Architectural blocker? | Verdict |
|---|---|---|
| Data tests | No | **Shipped** (`// data_test`) |
| Incremental materializations | No, but pick a philosophy | TODO with design decision |
| Column lineage + docs site | No | Pure TODO |
| Snapshots / SCD2 | No | New output kind |
| Selective execution grammar | No | UI/CLI surface |
| Schema contracts | No, but design metadata model | TODO with design work |
| Packages / community | Closed annotation parser starts to bind | Decide extensibility model |
| Semantic layer / metrics | No | Large additive scope |

The three items where the current abstraction needs deliberate decisions
before more weight lands on it: **incremental philosophy, schema metadata,
and annotation extensibility**. The rest is execution.

### 1. Data tests

dbt: `unique`, `not_null`, `accepted_values`, custom generic tests, plus
singular tests. Run as `SELECT` statements that pass when they return 0 rows.

**Shipped** via the `// data_test` annotation (built on materialization):
`// data_test unique <col>`, `not_null`, `accepted_values <col> = a,b,c`,
`relationships <col> -> <asset>.<col>`, and `// data_test <script_path>` for
the custom escape hatch (dbt's singular test). Each compiles to a SQL
*verifier probe* that runs against the freshly-materialized asset and raises on
violation, riding the existing failure-propagation path. This is also the first
*extensible* annotation — see `ducklake-materialization.md` §"Data tests" for
the annotation→verifier pattern that column-lineage will reuse. The keyword is
`data_test`, not `test`, to stay clear of the unrelated `// test:` CI-test
annotation.

### 2. Incremental materializations

See [Incremental deep-dive](#incremental-deep-dive) below.

### 3. Column lineage + docs

dbt: SQL-AST parsing for column-level deps; `dbt docs serve` produces a
static lineage site with descriptions.

Today: graph is asset-level. `SqlQueryDetails` in the parser
(`backend/parsers/windmill-parser/src/asset_parser.rs:44`) already has a
column map — the scaffolding exists. No `// column` annotation, no docs
surface. Pure TODO; no abstraction stands in the way.

### 4. Snapshots / SCD2

dbt: `{% snapshot %}` blocks with `strategy='timestamp'` or `'check'`.
Today: nothing. Add as a new `PipelineOutputKind` + `// snapshot strategy=
timestamp updated_at=updated_at unique_key=id` annotation. Same shape as
other output kinds.

### 5. Selective execution grammar

dbt: `--select tag:nightly+ state:modified+ +my_model+`.
Today: `requestRunCascadeSignal` in the canvas, `// tag` annotation parsed.
Graph + tags + last-run state has all the inputs. UI/CLI surface, not
abstraction work.

### 6. Schema contracts

dbt: `contract: enforced` + `columns: [{name, data_type}]`. Compile-time
check that model output matches the declaration.

Today: `// on datatable://users/active` is a string. Rename a column
upstream → downstream breaks at runtime, silently.

This is the item where the current asset abstraction is thinnest.
To do contracts well: capture output schemas after a run (substrate-specific
DESCRIBE), persist them as asset metadata, validate consumer references at
save time. The asset-as-typed-node model accommodates it — but **where**
schemas live (asset row, sidecar?), **when** they're captured (post-run?
edit-time?), and **how** versioning works are non-trivial design choices.
Worth doing intentionally now while the asset surface is still young.

### 7. Packages / community

dbt: `dbt deps`, `dbt-utils`, `dbt-expectations`. Whole ecosystem on Jinja
macros.

Today: closed-vocabulary annotation parser — `parsePipelineAnnotations`
hardcodes `pipeline`, `partitioned`, `freshness`, `trigger`, `debounce`,
`tag`, `retry`, `on`. No way for a package to register
`// test rows_between 100 1000000` or `// hook on_failure my_alert`.

This is the one place the current abstraction starts to bind. Macros are
also dbt's biggest pain source — we don't have to replicate them. Possible
shapes:

- **Hooks-as-scripts**: `// on_failure f/lib/alert`, `// pre_run f/lib/setup`.
  Value is a script path. Stays inside the closed annotation set; new
  hook *types* still require parser changes but third-party *behavior*
  ships as scripts.
- **Test types as scripts**: a test is a script that returns 0/1, packaged
  via the hub like anything else.
- **Materialization plugins**: harder; template-generator would need to be
  extensible.

Doing this *after* you've shipped 30 hardcoded annotations is much harder
than doing it now.

### 8. Semantic layer / metrics

dbt: `metrics:` blocks, MetricFlow, BI-tool query API. Large scope,
additive. Lowest priority of the eight.

## Incremental deep-dive

### How dbt incremental works

```sql
-- models/marts/orders_daily.sql
{{ config(
    materialized='incremental',
    unique_key='order_id',
    incremental_strategy='merge',
    on_schema_change='append_new_columns'
) }}

SELECT order_id, user_id, amount, created_at
FROM {{ ref('orders_raw') }}
{% if is_incremental() %}
WHERE created_at > (SELECT MAX(created_at) FROM {{ this }})
{% endif %}
```

- **First run** (target doesn't exist): `CREATE TABLE orders_daily AS
  SELECT ...` — full build, no WHERE.
- **Subsequent runs**: stage to temp table, then MERGE on `unique_key`.

Knobs: `incremental_strategy` ∈ {`merge`, `append`, `delete+insert`,
`insert_overwrite`, `microbatch`}. `on_schema_change` ∈ {`fail`, `ignore`,
`append_new_columns`, `sync_all_columns`}. `--full-refresh` forces rebuild.

Pain points: watermark + unique_key interaction is subtle (late-arriving
rows past the watermark are silently dropped); `on_schema_change` defaults
to `ignore` (silent column drop); Jinja `is_incremental()` runs at compile,
not runtime — debugging requires `dbt compile`; cross-warehouse MERGE
dialect is dbt's biggest internal complexity.

### Where Windmill stands today

- `// partitioned daily|hourly|weekly|monthly|dynamic key=...` parsed into
  `PartitionSpec` at
  `backend/parsers/windmill-parser/src/asset_parser.rs:172`.
- `// freshness 1h` parsed.
- Templates emit `CREATE TABLE IF NOT EXISTS ... AS SELECT *` — full
  refresh, every run, no partition substitution.
- No `WM_PARTITION_*` context flowing into scripts.
- No materialized-partition state per asset.

Annotations are present but metadata-only. Nothing actually executes
incrementally yet.

### Path A — Literal templates ("script is the truth")

Philosophy: WYSIWYG. Windmill never wraps. Templates scaffold boilerplate,
partition context is injected as bind / env vars, the user owns the SQL.

```sql
-- pipeline
-- on datatable://prod/orders_raw
-- partitioned daily
-- unique_key order_id

ATTACH 'datatable://prod' AS pg;

CREATE TABLE IF NOT EXISTS pg.orders_daily (
  order_id    BIGINT PRIMARY KEY,
  user_id     BIGINT,
  amount      NUMERIC,
  created_at  TIMESTAMPTZ
);

CREATE OR REPLACE TEMP TABLE _stage AS
SELECT order_id, user_id, amount, created_at
FROM pg.orders_raw
WHERE created_at >= $WM_PARTITION_START
  AND created_at <  $WM_PARTITION_END;

BEGIN;
DELETE FROM pg.orders_daily
 WHERE created_at >= $WM_PARTITION_START
   AND created_at <  $WM_PARTITION_END;
INSERT INTO pg.orders_daily SELECT * FROM _stage;
COMMIT;
```

Runtime: resolve `(value, start, end)` from scheduler tick / trigger event
/ backfill range → bind as SQL params → execute script as-is → record
`(asset_path, partition_value)` on success.

**Pros**: no compile step; backfill is trivial (idempotent DELETE+INSERT);
late-arriving data → just re-run the affected partition; no dialect
rewriting in core; Python/TS/Bash/SQL all fit the same model.

**Cons**: boilerplate per script; materialization changes require script
edits; user owns dialect specifics.

### Path B — dbt-style wrapping

Philosophy: separate intent (SELECT) from execution (DDL). User declares
what; Windmill compiles to per-substrate DDL.

User writes:

```sql
SELECT order_id, user_id, amount, created_at
FROM pg.orders_raw
WHERE created_at >= $WM_PARTITION_START
  AND created_at <  $WM_PARTITION_END
```

Runtime parses, looks up target schema, wraps per output kind +
strategy + first-run/subsequent-run state.

**Pros**: concise; materialization is a config flip; automatic schema-drift
handling; cross-substrate consistency.

**Cons**: two-layer execution ("what ran?" needs a compile-output view);
SELECT-only restricts pre/post-statement work (dbt's answer: `pre_hook` /
`post_hook` — more surface); doesn't generalize to Python/TS (you end up
with two execution models); cross-substrate MERGE dialect is where dbt has
burned the most engineering — we'd inherit that tax forever; schema
introspection per substrate is its own project.

### Path C — Hybrid (recommended)

- **Literal-by-default**: scaffolds emit full DDL with `WM_PARTITION_*`
  substitution. WYSIWYG for all languages.
- **Helper library** (e.g. `wmll.partition`, `wmll.datatable.upsert_partition`):
  lifts boilerplate into library calls without hiding semantics — readable
  source.
- **Opt-in wrapping** for single-SELECT SQL scripts via
  `// materialized incremental wrap=true`. Limit to DuckDB first; add
  others as needed. Always log the compiled SQL.
- **State + backfill UI**: persist materialized partitions per asset; UI
  to backfill a range with concurrency cap.

Ships A's 80% case first without committing to B's dialect-rewriting tax.
Wrapping becomes opt-in convenience for users who want dbt-style ergonomics.

> See [`ducklake-materialization.md`](./ducklake-materialization.md) for the
> DuckLake-native realization of this path: how snapshots make the assets
> versioned/reproducible for free, the executor codegen seam, and the
> materialization-metadata schema.

### Decisions either path forces

1. **Partition window provenance.** Scheduler tick? Trigger event time
   (Kafka `event_time` header)? Explicit backfill? Default = "now's bucket"?
2. **Surface.** Bind params (`$WM_PARTITION_START`), env vars
   (`WM_PARTITION_START`), helper library — probably all three for
   different languages, but pick canonical names.
3. **First-run bootstrap.** Template scaffolds `CREATE TABLE IF NOT
   EXISTS` (A), or runtime detects "table missing → full refresh" (B).
4. **State tracking.** `materialized_partitions` keyed by `(workspace,
   asset_kind, asset_path)`. Drives "run stale," backfill gap detection,
   downstream waiting.
5. **Backfill execution.** N partitions → serial? Parallel with
   concurrency cap per asset?
6. **Idempotency contract.** `// partitioned` should imply "re-running the
   same partition is safe." Templates and helpers must enforce.

## Partitioning vs. incremental: the reframing

Partitioning covers ~80% of what dbt's incremental does. What it gives
for free:

- Unit of work (one partition per run)
- Idempotency (DELETE-by-partition + INSERT is safe to rerun)
- State (track which partitions are materialized)
- Backfill (re-run a range)
- First-run vs. subsequent-run (every run is "process partition P" — no
  special case)
- "Process only new data" (the partition window IS the filter)

dbt itself has been migrating toward partition-first thinking via
`microbatch` strategy — essentially `incremental` with mandatory partition
key.

### What partitioning alone doesn't address

**Dedup within a partition by a separate key.** Example: partition by
`created_at` daily, but `orders_raw` is mutable — the same `order_id` can
appear multiple times in one partition (initial create, then amendments).
You want `orders_daily` to hold the latest version per `order_id`.

DELETE-by-partition + INSERT works only if you reprocess from a
source-of-truth source. If you're consuming amendments and need dedup
*within* the slice, you need MERGE on `order_id`, not DELETE on partition.

This is what dbt's `unique_key` does. Orthogonal to partitioning:
`partitioned` answers "which slice?"; `unique_key` answers "how do I dedup
inside the slice?"

**Pure watermark-based incremental.** Mostly subsumed by
`// partitioned dynamic key=updated_at` — a partition becomes "everything
since the last seen value of `key`."

### Recommended annotation shape

Don't build "incremental" as a concept. Build:

- `// partitioned <kind>` — unit of work + state + backfill (already exists).
- `// unique_key <col>` — opt-in dedup-within-partition. Drives MERGE
  template vs. DELETE+INSERT template.
- `// append` — opt-out of dedup entirely (INSERT-only, no DELETE).

This collapses dbt's `materialized=incremental` + `incremental_strategy` +
`unique_key` into orthogonal annotations that compose. Partition-first is
the better mental model.

Schema drift handling (`on_schema_change`) is genuinely separate — applies
to full-refresh too — and belongs with the schema-contracts work (gap #6).

## First implementation slice

Sequencing if we go with the hybrid + partition-first reframing:

1. **Partition runtime context** — resolve `(value, start, end)` from
   scheduler / trigger / backfill, surface as bind vars + env vars. No
   materialization change yet.
2. **Helper library** — `wmll.partition.window()`,
   `wmll.datatable.upsert_partition()` for Python/TS, SQL macros for
   DuckDB/PG.
3. **Template updates** — when `// partitioned X` is present, scaffold
   DELETE+INSERT (or MERGE when `// unique_key` also present, or INSERT
   when `// append`).
4. **Materialized-partition state** — new table keyed by
   `(workspace, asset_kind, asset_path, partition_value)`. Asset metadata
   read API exposes it.
5. **Backfill UI** — date range picker on the pipeline folder page; fans
   out runs with concurrency cap.
6. *Later, behind a flag:* opt-in wrap mode for single-SELECT DuckDB.

Delivers dbt's pragmatic value (incremental, backfill, idempotent reruns)
without buying the compile-layer maintenance, and keeps Windmill
recognizably Windmill-shaped.
