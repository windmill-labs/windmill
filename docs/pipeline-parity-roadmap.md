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
| 1 | Data tests (+ extensibility pattern #6) | A | 1 | `pipeline-data-tests` | 🟡 in progress |
| 3 | Snapshot pinning | B | 1 | `pipeline-snapshot-pinning` | 🟡 in progress |
| 4 | Selective execution grammar | — | 1 | `pipeline-selective-execution` | 🟡 in progress |
| 2 | Schema contracts / drift | B | 2 | — | ⏸ blocked on #3 |
| 5 | Column lineage + docs surface | A | 2 | — | ⏸ blocked on #1 |
| 6 | Annotation extensibility | A | — | folded into #1 | n/a |
| 7 | Semantic layer / metrics | — | later | — | 🔵 deferred |
| — | SCD2 snapshots (dbt `{% snapshot %}`) | — | — | — | ⛔ won't build (DuckLake time-travel supersedes) |

Legend: 🟡 in progress · ⏸ blocked · 🔵 deferred · ✅ merged · ⛔ won't build

## Dependency graph

```
Round 1 (parallel — disjoint surfaces):
  #1 data tests ........ Surface A
  #3 snapshot pinning .. Surface B
  #4 selective exec .... independent

Round 2 (each unblocks when its round-1 sibling on the SAME surface merges):
  #1 ──merges──▶ #5 column lineage   (same parser pair; follows #1's extensibility convention)
  #3 ──merges──▶ #2 schema contracts (same capture path; reuses #3's snapshot/DESCRIBE capture)
```

**Why round 2 is not parallel with round 1:**

- **#2 schema contracts** extends the same post-run capture in `materialization.rs` /
  `duckdb_executor.rs` that #3 modifies, and wants to persist post-run `DESCRIBE`
  output as asset metadata — naturally built on #3's capture extension. Running
  concurrently = conflicts in Surface B + duplicated capture work.
- **#5 column lineage** adds a `// column` annotation to the same parser pair #1
  touches, and should follow the test-as-annotation extensibility convention #1
  establishes (#6) rather than re-deriving it. Running concurrently = conflicts
  in Surface A.

**Base round-2 branches off the merged round-1 branch, not bare `main`**, so they
inherit the groundwork (capture extension / annotation pattern) instead of
rebasing onto it later.

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

## Round 2 — queued (not yet started)

### #2 schema contracts / drift (Surface B, base off #3)
Capture output schemas after a run (substrate-specific `DESCRIBE`), persist as
asset metadata, validate consumer `// on datatable://…` references at save time.
Closes the thinnest seam in the abstraction (a bare ref string today → silent
runtime break on upstream rename). Also absorbs dbt's `on_schema_change`
(schema-drift handling applies to full-refresh too). Design-open: where schemas
live (asset row vs. sidecar), when captured (post-run vs. edit-time),
versioning. Ref: `pipelines-vs-dbt.md` §6 + §"Partitioning vs. incremental"
closing note.

### #5 column lineage + docs surface (Surface A, base off #1)
`// column` annotation + column-level lineage view; `SqlQueryDetails` already
carries a column map (`asset_parser.rs:44`). Follow #1's extensibility
convention. Ref: `pipelines-vs-dbt.md` §3.

## Merge checklist (run when a round-1 PR merges)

1. Flip its row to ✅ in the status board; record the PR number.
2. Unblock its round-2 dependent (⏸ → ready) per the dependency graph.
3. Spawn the round-2 session **based off the just-merged branch**, e.g.
   `webmux add pipeline-schema-contracts --base <merged-#3-branch> --detach --prompt …`.
4. If both round-1 surface-owners (#1 and #3) have merged, round 2 (#2 + #5) can
   itself run in parallel — they sit on different surfaces again.

## Deferred / won't-build

- **#7 semantic layer / metrics** — large additive scope, lowest priority; revisit
  after parity gaps close.
- **SCD2 snapshots** — deliberately not built. DuckLake time-travel (via #3) is a
  strictly better answer for most of dbt's `{% snapshot %}` use; one fewer engine
  to maintain (`ducklake-materialization.md` §"Reproducibility").
