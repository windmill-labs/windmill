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
| Column lineage | No | **Shipped** (`// column`); docs site still TODO |
| Snapshots / SCD2 | Yes (`key=… history`) | Managed strategy |
| Selective execution grammar | No | UI/CLI surface |
| Schema contracts | No, but design metadata model | **Shipped** (capture #2a + save-time check #2b) |
| Packages / community / macros | No | **Shipped** (`// macros` workspace macro libraries) |
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

Enterprise goes one step *beyond* dbt here: write-audit-publish. dbt commits
the model then tests it, so a failing test leaves the bad table live for BI
readers; the enterprise build runs the same probes inside the write
transaction and rolls back on violation, so a failing slice is never
published (see `ducklake-materialization.md` §"Scoping decisions").

### 2. Incremental materializations

See [Incremental deep-dive](#incremental-deep-dive) below.

### 3. Column lineage + docs

dbt: SQL-AST parsing for column-level deps; `dbt docs serve` produces a
static lineage site with descriptions.

**Shipped**, inferred-first. For DuckDB scripts the column lineage is **derived
automatically from the SQL AST** — `windmill-parser-sql-asset` walks each
output-producing query's projection and maps every output column to the source
columns its expression reads (passthroughs *and* computed columns like
`amount + tax AS total`), resolving each input to its asset via the same
`ATTACH`/alias machinery the asset parser already uses. This is the dbt-style
AST lineage, and it needs no annotation.

The `// column <out_col> <- <asset-uri>.<col>[, …]` annotation is the
**override / escape hatch**, for the cases inference can't reach: polyglot
transforms (Python/TS/Bash — no SQL AST), dynamic SQL (`${sql.raw(...)}`, flagged
by `SqlQueryDetails.has_raw_interpolation`), or correcting a mis-inferred edge.
Inferred and annotated lineage are merged per output column with the annotation
winning (`merge_column_lineage`). The annotation is the second *extensible*
annotation family after `// data_test` — same head-then-tail parse shape (see
`ColumnLineage`/`ColumnRef` in `asset_parser.rs`) — and is pure metadata: it
drives the graph surface, never a runtime probe.

Surfaced two ways in the asset graph: a count badge on the
producer→materialized-asset write-edge, and a **transitive column-lineage
trace** (`ColumnLineageTrace.svelte`, over the cross-script graph built by
`columnLineageGraph.ts`) in the asset details pane — select an asset to see its
columns and their full upstream/downstream lineage, click any column to
highlight its complete impact set across the pipeline (forward + backward).

SQL-AST inference runs both **server-side** (the graph endpoint + deploy via
`parse_assets`, for *deployed* members) and **in the live editor** — the same
parser compiled to WASM (`windmill-parser-wasm-asset`) runs on the open draft's
buffer, and `resolveGraph` merges its `column_lineage` with the buffer's
`// column` annotations under the same annotation-wins precedence, so the draft
preview matches what deploys. The `dbt docs serve`-style static lineage *site*
is still TODO.

### 4. Snapshots / SCD2 — **shipped**

dbt: `{% snapshot %}` blocks with `strategy='timestamp'` or `'check'`.
Shipped as a modifier on the keyed merge: `// materialize ducklake://<lake>/<dim>
key=<natural_key> history [track=<c1,c2,…>]` (the leading keyword `scd2` is an
alias). The SELECT returns the current snapshot (one row per key); the runtime
adds `valid_from`/`valid_to`/`is_current` and, on each run, diffs against the live
current rows to close changed versions and open new ones (dbt's `strategy='check'`
shape — `track=` is the check-columns list; empty ⇒ all non-key columns). Because
it's a managed strategy (not a new output kind), it inherits `// data_test` and
schema capture, and each run (re)creates a `<dim>_current` view; consumers get
effective-dated joins via native `ASOF JOIN … >= valid_from`. Deletes follow dbt:
soft by default (absent keys stay current), or `deletes=close` for
`hard_deletes=close`. v1 is non-partitioned; partitioned history is a follow-up.
Both misconfigurations — `history` without `key=`, and `history` + `//
partitioned` — are rejected **at deploy** (fail-fast at save, not on the first
run). See `ducklake-materialization.md`.

### 5. Selective execution grammar

dbt: `--select tag:nightly+ state:modified+ +my_model+`.
Today: `requestRunCascadeSignal` in the canvas, `// tag` annotation parsed.
Graph + tags + last-run state has all the inputs. UI/CLI surface, not
abstraction work.

### 6. Schema contracts — **shipped**

dbt: `contract: enforced` + `columns: [{name, data_type}]`. Compile-time
check that model output matches the declaration.

**Shipped**, capture-then-validate (no declaration to maintain): the schema a
managed `// materialize` run captures post-DESCRIBE into the versioned
`materialized_asset_schema` sidecar (#2a) *is* the contract, and every
consumer save/deploy is validated against the latest capture. Three surfaces,
same diff (`windmill_common::schema_contracts`, mirrored 1:1 in
`schemaContracts.ts`):

- **Save-time (authoritative)**: `POST /w/{ws}/scripts/check_schema_contracts`
  runs on the deployed content right after a save and returns **warnings —
  never errors**. A deliberate upstream reshape must not fail every consumer
  save; that's why dbt's `on_schema_change=fail` is deliberately not offered
  in v1 (a CI/CLI gate can layer it on later without touching the grammar).
- **Editor flycheck**: the WASM parse that already runs on the open buffer
  feeds the same diff, surfacing mismatches as live Monaco warning squiggles
  anchored to the offending read / `// column` / `// data_test` line.
- **Autocompletion**: annotation refs (`// column out <-
  ducklake://lake/orders.`, `// data_test relationships … ->`) complete
  column names from the captured schema — the broken ref never gets typed.

What's checked (ducklake-only — the only substrate with capture in v1;
datatable refs have no captured schema and stay silent): body-read/written
columns missing from the capture, `// column` lineage sources, and
`// data_test relationships` refs — including a captured-type *difference* on
the join columns when both sides are captured (phrased "differs", since the
runtime probe's IN-subquery still coerces). Column names compare
case-insensitively (DuckDB unquoted-identifier semantics); the managed
`_wm_partition` column is whitelisted; a `<dim>_current` scd2 view falls back
to its base table's capture (identical columns by construction).

The producer-side escape hatch is `// materialize … on_schema_change=ignore`:
it declares the schema deliberately unstable and collapses downstream
warnings for that asset into a single informational note. Default is `warn`.

### 7. Packages / community / macros — **shipped** (macro libraries)

dbt: `dbt deps`, `dbt-utils`, `dbt-expectations`. Whole ecosystem on Jinja
macros — text templating that exists largely to paper over weak warehouse
SQL and absent code reuse.

**Shipped** as workspace **DuckDB macro libraries** — engine-native
`CREATE MACRO` (scalar + table-valued) instead of templating, resolved and
injected by Windmill ("resolve-and-inject"):

- A DuckDB script annotated `// macros` is a library: its body is
  `CREATE OR REPLACE MACRO` statements (plus plain setup). Deploy parses each
  macro into the `macro_definition` registry (workspace-unique names) —
  available to consumers the moment the deploy transaction commits.
- Any DuckDB script that *calls* a registered macro gets
  `CREATE OR REPLACE TEMP MACRO …` blocks injected into its statement list at
  job time — dependency-topo-ordered (DuckDB bind-checks macro bodies at
  CREATE) and placed after the setup/ATTACH prefix. Each provider library's
  own setup statements are injected ahead of the definitions (deduped across
  libraries), so a macro body that references its lib's ATTACH binds on the
  implicit path too. Late-bound: a lib redeploy applies to subsequent runs —
  workers cache the registry per workspace and evict it via a transactional
  `notify_macro_registry_change` event (worst-case staleness = the notify
  poll interval, ~10s; 60s TTL as backstop) — dbt's semantics with a
  deploy-time registry instead of a compile step.
- `// use <lib_path>` force-injects a whole library (definitions + its setup
  statements) — the escape hatch for dynamic SQL (e.g. calls inside
  `query('…')` strings) that lexical detection can't see. A library's own
  `// use` declarations are honored transitively: consuming a macro from lib B
  pulls in whatever B `// use`s, so a dynamic dependency stays encapsulated in
  the library instead of leaking to every consumer.
- Deploy-time validation: builtin-shadow rejection (DuckDB silently allows
  shadowing `concat`!), within-file forward references, cross-lib name
  collisions, non-setup statements, managed-ATTACH setup.
- Graph surface: the library renders as a node ("defines N macros" badge +
  signature strip in the details pane) with violet lib→consumer edges from
  deploy-recorded call detection (`macro_usage`) and `// use` annotations.

**Trust model (deliberate, dbt-package-like).** Macro libraries are
workspace-trusted code: anyone who can deploy a script can publish macros,
and implicit call detection injects them into other users' DuckDB jobs with
the *consuming* job's credentials — the same trust boundary as a dbt package
in the project (any committer changes every model). Two guardrails bound it:
a script's **own** macro definitions always win (a library deploy can never
replace a local `CREATE MACRO` — the name is excluded from injection
entirely), and names are workspace-unique with built-in shadowing rejected
at deploy. A call to a macro that doesn't exist yet is the residual surface
(whoever first deploys a lib defining that name "captures" the call) — if
workspaces ever need finer trust, per-consumer opt-in/opt-out or
ACL-filtered injection can be layered on the same seam.

Table macros + `query_table(src)` cover most of dbt's *model factory*
pattern (one parameterized transform, N thin per-asset scripts); DuckDB's
richer SQL (`COLUMNS(*)`, native `PIVOT`, lambdas) absorbs much of
dbt-utils outright. Parameterized custom data tests (dbt's generic tests)
remain a follow-up on the `// data_test <script>` seam, as does the broader
annotation-extensibility question (hooks-as-scripts etc.) — macros closed
the reuse gap without opening the parser vocabulary.

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

Note the boundary: the keyed merge reconciles the incoming rows against the
*target* (delete-by-key + insert), but it does not deduplicate the *source*.
"Latest version per `order_id`" is still your SELECT's job — pick one row per
key (e.g. `QUALIFY row_number() OVER (PARTITION BY order_id ORDER BY updated_at
DESC) = 1`). A source that still has two rows for one key is rejected at run
time rather than double-written (see `ducklake-materialization.md`).

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
