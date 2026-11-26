# Pipeline parity roadmap

Living execution tracker for closing the dbt/dagster parity gaps on the
DuckLake pipeline/materialization system. Updated as PRs merge.

- **What we're matching/beating:** [`pipelines-vs-dbt.md`](./pipelines-vs-dbt.md)
  (the eight gaps, ranked).
- **The shipped substrate:** [`ducklake-materialization.md`](./ducklake-materialization.md)
  (annotation grammar, executor codegen seam, metadata schema).

This doc is the *plan of record* for how the remaining gaps get sequenced and
parallelized — not a re-analysis. If a gap's design changes, edit the source
doc above; if its *status or ordering* changes, edit here.

## Baseline (shipped)

`#9689 — feat: ducklake materialization for data pipelines` landed steps 1–5
of the DuckLake v1 slice:

- `sql_materialize.rs` classifier (managed/manual + `key=`/`append` strategies).
- DuckLake DELETE+INSERT codegen with physical `_wm_partition` wiring.
- `materialized_partition` state table, **with `snapshot_id` captured per row**.
- Partition-status grid + backfill range dialog (CE observability).

So managed/incremental/versioned assets, idempotent partitioned reruns, the
backfill substrate, and per-asset materialization observability all exist.
**Not yet wired:** snapshot pinning (the id is recorded but never threaded to
consumers — no `WM_UPSTREAM_SNAPSHOT` anywhere), and all eight parity gaps.

## The two shared hot-surfaces

Every sequencing decision below follows from which of these two surfaces a task
touches. Two tasks on the same surface **cannot** run concurrently without
guaranteed merge conflicts.

**Surface A — annotation vocabulary (dual parser, kept in lockstep):**
- `frontend/src/lib/components/assets/AssetGraph/parsePipelineAnnotations.ts`
  (drives live graph preview)
- `backend/parsers/windmill-parser/src/sql_materialize.rs` (drives deploy)
- guarded by `parsePipelineAnnotations.parity.test.ts` — adding a field to
  `PipelineAnnotations` is a compile error until its parity assertion is wired,
  so both parsers move together.

**Surface B — materialization capture / codegen:**
- `backend/windmill-common/src/materialization.rs`
- `backend/windmill-worker/src/duckdb_executor.rs`
- `backend/windmill-api-assets/src/lib.rs`

**Neither surface (independent):** the asset graph traversal + run/backfill
trigger UI/CLI — `graphTraversal.ts`, `resolveGraph.ts`, `cascadeOrchestrator.ts`,
`cli/src`.

## Status board

| # | Gap | Surface | Round | Branch / PR | Status |
|---|---|---|---|---|---|
| 1 | Data tests (+ extensibility pattern #6) | A | 1 | **#9708** | ✅ merged |
| 3 | Snapshot pinning | B | 1 | **#9709** | ✅ partial — time-travel reads shipped; auto cascade-pin deferred |
| 4 | Selective execution grammar | — | 1 | #9695 + `pipeline-selective-execution` | 🟡 base merged; `+asset`/`asset+`, `tag:`, `state:modified+` still open |
| 2a | Schema **capture** (DESCRIBE → asset metadata) | B | 2 | — | ▶ ready (off `main`) |
| 2b | Schema **contract enforcement** (save-time, cross-node) | B | 2 | — | ⏸ blocked on #5 |
| 5 | Column lineage + docs surface | A | 2 | — | ▶ ready (off `main`) |
| 6 | Annotation extensibility | A | — | folded into #1 (✅ shipped in #9708) | n/a |
| 7 | Semantic layer / metrics | — | later | — | 🔵 deferred |
| — | SCD2 snapshots (dbt `{% snapshot %}`) | — | — | — | ⛔ won't build (DuckLake time-travel supersedes) |

Legend: ▶ ready · 🟡 in progress · ⏸ blocked · 🔵 deferred · ✅ merged · ⛔ won't build

> **Status sync (round 1 merged).** #1 landed as **#9708** (full annotation→verifier
> seam across both parsers + `duckdb_executor.rs`). #3 landed as **#9709** but only
> the *explicit* time-travel read UX (`query_builders.rs` `DESCRIBE … AT (VERSION => n)`
> + history panels) — it did **not** add a post-run schema-capture seam and did **not**
> touch `asset_dispatch.rs`, so automatic cascade pinning (`$WM_UPSTREAM_SNAPSHOT`)
> stays deferred. Consequence below.
>
> **#2 was wrong to assume it "bases off #3's capture path".** #9709 added no capture
> seam, so #2's schema capture is **greenfield** (it can reuse #9709's
> `DESCRIBE … AT (VERSION => n)` helper, but builds its own post-materialize capture).
> #2 is therefore split:
> - **#2a schema capture** — DESCRIBE the materialized output, persist as asset
>   metadata. Surface B, independent → **ready now off `main`**, parallel-safe with #5
>   (A vs B; only shared touch-point is `windmill-api-assets/src/lib.rs` — keep edits
>   to distinct fns).
> - **#2b contract enforcement** — save-time validation of *consumer* `// on
>   datatable://…` refs + type/shape + `on_schema_change`. This needs to know which
>   columns each consumer **reads**, i.e. it depends on **#5 column lineage** → base
>   #2b off merged #5, not `main`.
>
> **#2 must NOT re-implement value-level checks already shipped by #1 (#9708):**
> `not_null`, `accepted_values`, `unique`, `relationships` are data-quality probes that
> already exist. Contracts = the part `data_test` structurally can't be: *save-time*
> (not post-run), *cross-node* (producer→consumer edge, not single asset), and
> *type/shape* (not row values). The producer's own "output matches declared schema"
> self-check should ride #1's verifier seam, not a new mechanism.

## Dependency graph

```
Round 1 (MERGED): #1 (#9708, Surface A) · #3 (#9709, Surface B, partial) · #4 (#9695 base)

Round 2 — now that #1 + #3 are in `main`:
  #5 column lineage ...... Surface A, ready off `main`  (reuses #9708's annotation→verifier seam)
  #2a schema capture ..... Surface B, ready off `main`  (greenfield; reuses #9709's DESCRIBE helper)
        │
        └─#5 merges─▶ #2b contract enforcement  (needs #5's per-consumer column reads → base off merged #5)
```

#5 and #2a sit on different surfaces (A vs B) and both base off `main`, so they
run **in parallel now**. #2b is the one true sequential edge: its save-time
consumer-ref validation can't be written until #5 tells it which columns each
consumer reads.

**Base #2b off the merged #5 branch, not bare `main`**, so it inherits the
column-read lineage instead of stubbing it.

## Round 1 — scope per session

### #1 `pipeline-data-tests` (Surface A)
`// test unique|not_null|accepted_values|relationships <args>` + `// test <script_path>`
escape hatch. Tests execute against the freshly-materialized asset (post
DELETE+INSERT) and fail the run on violation. Must wire each new field in **both**
parsers + the parity test. Establishes the extensible test-as-annotation pattern
(#6) for #5 to follow. Ref: `pipelines-vs-dbt.md` §1, §7.

### #3 `pipeline-snapshot-pinning` (Surface B)
Thread each producer's captured `snapshot_id` into the cascade trigger blob;
expose `$WM_UPSTREAM_SNAPSHOT` to consumers and emit `AT (VERSION => …)` reads.
Converts the recorded-but-dead integer into reproducible reads + rollback +
"state at the failing run" debugging. Open decisions to resolve in-session:
pin scope (direct producers vs. full transitive upstream), multi-upstream
precedence, backfill pinning, snapshot retention vs. live pins
(`ducklake-materialization.md` §"Open decisions" 2–3). Ref:
`ducklake-materialization.md` §"Reproducibility".

### #4 `pipeline-selective-execution` (independent)
dbt-style `--select` node selector (`tag:`, `state:modified+`, `+asset`/`asset+`
graph ancestors/descendants) over the existing graph + tags + `materialized_partition`
state. CLI (`wmill`) + asset-graph UI. **Read-only over tags — must not modify
either parser.** If a CLI command changes, run `python system_prompts/generate.py`.
Ref: `pipelines-vs-dbt.md` §5.

## Round 2 — ready (off `main`)

### #5 column lineage + docs surface (Surface A, base off `main`)
`// column` annotation + column-level lineage view; `SqlQueryDetails` already
carries a column map (`asset_parser.rs:44`). Follow #1's (#9708) extensible
test-as-annotation convention — wire each new field in **both** parsers + the
parity test. Ref: `pipelines-vs-dbt.md` §3.

### #2a schema capture (Surface B, base off `main`)
Capture the materialized output schema (substrate-specific `DESCRIBE`; reuse the
`DESCRIBE … AT (VERSION => n)` helper #9709 added to `query_builders.rs`) and
persist as asset metadata. Greenfield — #9709 added no capture seam. Design-open:
where schemas live (asset row vs. sidecar), when captured (post-run vs.
edit-time), versioning. Parallel-safe with #5 (A vs B); only shared file is
`windmill-api-assets/src/lib.rs` — keep edits to distinct fns. Ref:
`pipelines-vs-dbt.md` §6.

### #2b contract enforcement (Surface B, base off merged #5)
Validate consumer `// on datatable://…` references against captured producer
schemas at **save time** (a bare ref string today → silent runtime break on
upstream rename); add type/shape checks and absorb dbt's `on_schema_change`. The
producer's own "output matches declared schema" check rides #1's verifier seam —
do **not** rebuild a run-time probe mechanism, and do **not** re-implement #1's
value-level checks (`not_null`/`accepted_values`/`unique`/`relationships`).
Needs #5's per-consumer column reads → sequence after #5. Ref:
`pipelines-vs-dbt.md` §6 + §"Partitioning vs. incremental" closing note.

## Merge checklist (run when a round-2 PR merges)

1. Flip its row to ✅ in the status board; record the PR number.
2. When **#5** merges, unblock **#2b** (⏸ → ready) and spawn it **based off the
   merged #5 branch**, e.g.
   `webmux add pipeline-contract-enforce --base <merged-#5-branch> --detach --prompt …`.
3. `+asset`/`asset+`, `tag:`, `state:modified+` remain open under **#4** — land
   them on `pipeline-selective-execution` (independent surface), not a new cut.

## Deferred / won't-build

- **#7 semantic layer / metrics** — large additive scope, lowest priority; revisit
  after parity gaps close.
- **SCD2 snapshots** — deliberately not built. DuckLake time-travel (via #3) is a
  strictly better answer for most of dbt's `{% snapshot %}` use; one fewer engine
  to maintain (`ducklake-materialization.md` §"Reproducibility").
