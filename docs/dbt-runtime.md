# Windmill as a dbt runtime

Implementation spec for running an existing dbt project on Windmill with no
changes to the project itself. Companion to [`pipelines-vs-dbt.md`](./pipelines-vs-dbt.md),
which covers the opposite direction (native pipeline features that replace dbt).
The two are complementary: this is the adoption ramp, that is the long game.

Benchmark to beat is Airflow + [astronomer-cosmos](https://astronomer.github.io/astronomer-cosmos/),
the dominant way dbt is orchestrated today.

## Scope

- **In**: run an unmodified dbt project synced into Windmill, one Windmill job per
  invocation, live per-model observability, dbt models as first-class assets in
  the existing asset graph.
- **Out**: one Windmill job per dbt model, `state:modified` / slim CI,
  `dbt docs` hosting, semantic layer, dbt platform integration.
- **CE**: the runtime, the manifest ingest, the asset graph and every piece of
  UI ship in CE, as do all adapters except two. Only the `mssql` and `oracle`
  adapters are EE, mirroring the native `ScriptLang` boundary (decision 21).

## Decision log

| # | Decision | Resolution |
|---|---|---|
| 1 | dbt engine | Three-way toggle (`dbt-core-1x` \| `dbt-core-2x` \| `fusion`); shipped default `dbt-core-1x`, instance-configurable. See below |
| 2 | Artifact shape | `ScriptLang::Dbt` |
| 3 | Graph in v0 | Yes, both runtime and graph |
| 4 | Execution granularity | One job per invocation |
| 5 | Project storage | The project is the script's module bundle; nothing is cloned. See "Where the dbt project lives" |
| 6 | Multiple run configs | Per-run `select` on one script; N scripts means N projects |
| 7 | Run-time `select` | Descriptor default plus run-arg override |
| 8 | Credentials | Both `profiles.yml` passthrough and resource mapping |
| 9 | Adapter mappings | postgres, redshift, mysql, snowflake, bigquery, databricks; others via the project's own `profiles.yml` |
| 10 | Private repo auth | Not applicable: the project is synced, not fetched |
| 11 | Asset kind | `table://<resource>/<schema>/<name>`, not `dbt://`. See below |
| 12 | Graph refresh | Deploy-time, re-ingested per run only when the descriptor is dynamic. See below |
| 13 | Manifest storage | Sidecar table for nodes/edges. Full manifest **not** stored — see below |
| 14 | Metadata depth | Tests, strategy, tags, freshness, column descriptions. Column **lineage** is not in the manifest — see below |
| 15 | Node rendering | Asset nodes per model plus one runnable node for the script |
| 16 | Progress | Live, from the JSON event stream |
| 17 | Test failures | Honor dbt's own `severity` |
| 18 | Retry | Automatic node-level retry in-job, plus `dbt retry` as a run argument. See below |
| 19 | Caching | Worker-local global cache, keyed by the project digest |
| 20 | Images | Full images only |
| 21 | Licensing | CE except the `mssql` / `oracle` adapters. See below |
| 22 | Naming | Match Cosmos field names; importer deferred |

## Decision 1: engine toggle, and why the shipped default is not Fusion yet

`engine: dbt-core-1x | dbt-core-2x | fusion` in the descriptor, defaulting to an
instance setting so the default changes without a code change.

| Engine | Distribution | License |
|---|---|---|
| `dbt-core-1x` | Bundled: uv venv, `dbt-core` 1.12 plus adapter | Apache 2.0 |
| `dbt-core-2x` | Bundled: 47MB Rust binary | Apache 2.0 |
| `fusion` | **Never bundled.** Fetched from dbt Labs on first use, cached | dbt Fusion engine license agreement |

Fusion is the fastest option and the toggle exists so users can choose it. Two
things block making it the *shipped* default, both verifiable rather than matters
of taste:

1. **Redistribution terms.** The Fusion license grants only a "limited,
   non-exclusive, non-transferable, non-sublicensable" redistribution right, and
   4.1 forbids introducing "obstacles or delays that have the effect of hampering
   or interfering with (a) communication between Provider and End User, (b)
   User's ability to view, access, or use the Product and/or any Account
   Features." A sandboxed non-interactive job runner sits squarely in that
   clause's path, and "may not share, pool, or relay its own login credentials to
   any End User" reads directly onto putting one dbt platform token in a
   workspace secret. That needs counsel, not an engineering judgment.
   **Fetch-at-runtime is the mitigation**: the user's own instance pulls the
   binary from dbt Labs directly, so Windmill never redistributes and never
   interposes. Do not bake Fusion into any image.
2. **Fusion is v2 semantics, and v2 drops all deprecated functionality.** Every
   deprecation warning, including historic ones and those added in 1.10, must be
   resolved before a project runs on it. An arbitrary existing dbt 1.x project
   therefore may not run unchanged, which is this feature's entire premise. dbt
   ships an autofix tool and Fusion/Core interoperate side by side, so it is a
   migration users can do, but not one Windmill should silently require of them.

Consequence: ship with `dbt-core-1x`, which runs today's projects untouched, and
flip the instance default to `fusion` once counsel clears the runtime-fetch model
and a real project is verified end to end on it. Both bundled engines are
exercised by the e2e suite, so the flip is a config change, not a port.

## Decision 21: mirror the native warehouse boundary, do not invent one

Everything structural is CE: the executor, both bundled engines, the manifest
ingest, the `table://` asset graph, live progress, the editor. The only gate is
on two adapters, and it is not a dbt-specific policy — it is the same boundary
the native script languages already draw. Since `bigquery` and `snowflake`
became CE, the only warehouse `ScriptLang`s still behind a license are `mssql`
and `oracledb`, so those two dbt adapters are EE and every other one (postgres,
mysql, duckdb, snowflake, bigquery, databricks, redshift, clickhouse,
salesforce) is CE. Gating any of the others would make reaching a warehouse
through dbt stricter than reaching it natively, which is backwards.

Those two are *recognized* (for the gate and for the pip package the bundled
engine needs), not rendered from a resource: an `oracledb` resource is
`{user, password, database}` with no host/protocol/service, and dbt-sqlserver
needs an ODBC `driver` the images do not install. Both reach their warehouse
through the project's own `profiles.yml`, which is also how duckdb, clickhouse
and salesforce work.

The gate almost never fires in practice: `dbt-core-2x` supports neither adapter,
so it can only apply to `dbt-core-1x` with one of those two.

**The mechanism differs from the native languages.** They gate at compile time,
so a CE binary simply lacks the executor. That is not available here: there is
one dbt executor and the adapter is only known once the profile resolves. So it
is a runtime check on the resolved adapter, at both deploy and run, and it must
say what is wrong — a silent degradation that surfaces later as a connection
error is worse than no gate at all.

One trap: `ee_oss::LICENSE_KEY_VALID` is initialized to `true` in the OSS
variant, so reading it alone passes on a CE build. The check is
`cfg!(feature = "enterprise") && LICENSE_KEY_VALID`, which rejects both a CE
build and an enterprise build whose key did not verify.

## Decision 11: `table://`, not `dbt://`

`dbt://` is the intuitive choice and it quietly destroys the reason to build the
graph at all.

Asset identity has to be the **physical relation**, not the tool that produced it.
If a dbt mart registers as `dbt://analytics/orders_daily` while a native DuckDB
script reads `table://snowflake_prod/analytics/orders_daily`, those are unrelated
URIs, no edge forms, and dbt becomes an island in the graph. That is the
BashOperator outcome with extra steps. Keying on the relation is what makes the
existing cascade dispatch fire across the dbt boundary, which is the whole
differentiator.

So: `table://<resource_path>/<schema>/<name>`, one new `AssetKind`, resolved from
the profile's target so two scripts pointing at the same warehouse agree on
identity.

`dbt://` stays available as a namespace for dbt nodes that have **no** physical
relation and therefore cannot collide with anything: ephemeral models (inlined
CTEs, never written), exposures, and sources not separately modelled. Use it only
there, and only if those nodes prove worth rendering. Nothing uses it today.

Two traps, both of which quietly defeat the point if handled wrong.

**Identifier canonicalization.** `manifest.json` gives `relation_name`
pre-quoted (`"windmill"."Analytics"."Orders"`), an annotation is written by hand,
and the warehouses disagree on case: Snowflake folds unquoted identifiers up,
Postgres folds them down, DuckDB compares case-insensitively. Two spellings of
one table produce two nodes, no edge, and nothing looks broken in isolation. So
one rule is applied in exactly one place — `parse_asset_syntax`, the single
point where an asset URI becomes a graph key: strip the quote characters
(`"`, backtick, `[`/`]`) from the schema and name, then ASCII-lowercase them,
matching the case-insensitive identifier comparison the DuckDB paths already
use. The resource-path prefix is a Windmill path and stays case-sensitive.

**Warehouse identity is the Windmill resource path**, exactly as `datatable://`
and `ducklake://` do it — never the host, account or database. The resource names
the default database too, so it stays out of the key; a model that *overrides*
its database (Snowflake `database`, BigQuery `project`) is genuinely elsewhere and
qualifies its schema segment as `<database>.<schema>`, so two same-named relations
in different databases cannot collapse onto one node. When the default database is
unknown — the project brought its own `profiles.yml` — every relation qualifies,
because assuming they share one database is exactly what would collapse them.

Three call sites derive this key: the manifest ingest that creates the node, and
the live-progress and end-of-run paths that record status against it. They share
one function, because a site that derives it differently records progress against
a path no node has — the run still succeeds and the graph simply never moves. The same
warehouse is reachable under several hostnames, and credential material has no
business in an asset key. Accepted limitation, worth knowing before it is
filed as a bug: **two Windmill resources pointing at the same physical
warehouse do not unify**, so assets under one will not share edges with assets
under the other. Point both scripts at one resource to link them.

## Decision 12: the graph refreshes with the deploy

The project's files are the script's, so a deploy already sees exactly what will
run: it parses the bundle and stores the graph. "Refresh" is just "redeploy". No
manual button, no webhook, no separate mechanism.

The one case that cannot be settled at deploy is a descriptor that is dynamic by
construction: a `vars` value spelled with a `{{ placeholder }}`, or an `env`
value spelled `$var:` (re-resolved every run). dbt vars can steer `enabled`,
aliases, schemas, databases and materializations, so for those the deploy cannot
know what will run and the graph is re-ingested from every run's own manifest. Since asset dispatch fans out
from the stored rows, a run that cannot refresh them fails rather than cascading
from a stale graph — which also means those descriptors cannot run on an agent
worker, whose only DB access is through the API.

Agent workers are further limited: any dbt script whose profile Windmill resolves
is refused there, because the agent can neither read the relation root the graph
was ingested against nor re-ingest it — so a profile changed since the last
ingest would cascade from the wrong relations with nothing to detect it. A
project bringing its own `profiles.yml` has no root for Windmill to track and
runs there normally.

The refresh happens **before** the build, from a `dbt parse` with this run's own
vars and env — not after it. Dispatch fans out from the stored rows once the job
completes, so refreshing afterwards leaves a window in which those rows still
describe the previous run. The producer-writes cache is invalidated in-process
at the same moment rather than waiting for the notify poll, since the dispatch
for that job runs in the process that just refreshed.

Boundary that remains: the rows are keyed by script path, so **two concurrent
runs of one dynamic script race** — each overwrites the other's before either
dispatches, and a job can notify the other run's consumers. Give such a script a
concurrency limit of 1. Removing the race entirely means dispatching from a
per-job snapshot of what the run actually wrote, which is a change to the shared
cascade rather than to dbt.

Re-ingesting is nearly free: the run parses the project (about a second) before
building it and ingests that manifest.

The parse is what makes a newly added model appear in the same run that builds
it, rather than one run late: the graph is written before the build, so the
dispatch that follows sees this run's models.

## Where the dbt project lives

**In Windmill.** One dbt project is one Windmill script: the script's content is
the descriptor, and the project's files ride with it as its module bundle, a
path-keyed map the worker materialises into the job directory before invoking
dbt. There is one way to do this. Nothing is cloned, so there is no repository
resource, no ref, no commit and no clone cache.

A team whose repository must stay canonical keeps it: git-sync points at that
repository and pushes it into the workspace, so the repository still holds the
truth and Windmill receives the project. A team with no repository at all pushes
straight from a working copy.

### On disk, the project is a canonical dbt project

`wmill sync pull` writes the bundle verbatim, so the tree under the module folder
is exactly what dbt expects, with the extensions dbt expects:

```
f/analytics/
├── analytics.dbt.yaml              the descriptor (the script's content)
└── analytics__dbt/                 the module bundle: the project, unmodified
    ├── dbt_project.yml
    ├── packages.yml
    ├── models/staging/stg_orders.sql
    ├── models/marts/_marts__models.yml
    ├── macros/cents_to_dollars.sql
    ├── seeds/country_codes.csv
    └── snapshots/orders_snapshot.sql
```

Import is therefore a copy, never a transformation:

```
cp -r my-dbt-project/. f/analytics/analytics__dbt/
wmill sync push
```

Locally, dbt runs against the bundle with `--project-dir analytics__dbt` (or a
`cd`), which is what a monorepo holding several dbt projects already does, and
what dbt Cloud exposes as its "project subdirectory" setting.

**Why a module bundle rather than one script per model.** Models as scripts was
considered and rejected on three counts. A Windmill path admits no dots, and the
CLI rejects bare `.sql` as ambiguous (`.pg.sql`, `.duckdb.sql`, … are the
convention), so a model could only be typed by its location inside the project,
which breaks the rule that extension determines language. dbt resolves `ref()`
project-wide and cannot run a model alone, so each model job would reassemble
and reparse the whole project anyway. And `schema.yml` describes many models at
once, so splitting models into objects while their tests and docs stay in shared
YAML puts a model's contract in a different object. The bundle keeps the project
whole, and per-model execution is offered as an action on the graph node
(`--select <model>+`) rather than as a separate object.

What that costs, stated plainly: a model has no permissions or version history of
its own. The unit of both is the project.

### Consequences

**The version is the script version.** Deploying the script deploys the project
atomically; rollback is redeploying a previous version. The lockfile keeps the
resolved engine and adapter versions and the manifest digest.

**Windmill holds the files, so the graph can show them.** A model's compiled SQL
is readable from its node in the asset graph. Editing stays local: dbt
development is a CLI-and-editor loop (`dbt run --select`, `dbt test`, a local
warehouse), and a browser textarea over one file of a project is a worse version
of it. Windmill is the runner and the viewer.

**Two scripts against one project means two copies.** Splitting a project across
scripts, so an upstream selection and a downstream one compose, assumed a shared
repository. With bundles they would duplicate the project and drift. Prefer one
script per project with per-run `select`, and treat two scripts as two projects
(decision 6).

**Seeds are the only thing that can bloat a version.** Measured on real dbt code,
`.sql` files run about 500 bytes median and 1.9 KB at p90, so even a 5000-model
project is a few MB before compression. A single committed CSV can exceed all of
it, so the size guard names `seeds/` specifically rather than counting models.

## The script artifact

New `ScriptLang::Dbt`. Content is a YAML descriptor whose field names track dbt's
and Cosmos's vocabulary so the mental model ports without translation:

```yaml
engine: dbt-core-1x               # or dbt-core-2x | fusion
profile:
  resource: $res:f/prod/snowflake # rendered into profiles.yml
  target: prod
  # schema: marts                 # target schema; REQUIRED for BigQuery, whose
                                  # resource is a service-account JSON with no
                                  # dataset in it
  # profiles_yml: profiles.yml    # alternative: keep your own file, but then
                                  # there is no resource path to key assets on,
                                  # so the project gets no graph (see below)
select: ["tag:nightly+"]
exclude: []
test_behavior: build              # build | after_all | none
vars:                             # typed: numbers/bools/lists keep their type,
  run_date: "{{ run_date }}"      # and string leaves take job arguments
  strict: false
threads: 8
full_refresh: false
env:                              # for the project's own `{{ env_var() }}`
  DBT_PASSWORD: $var:u/rf/wh_password
```

`env` values spelled `$var:<path>` are resolved to that Windmill variable, so a
project keeping its own `profiles.yml` never needs a credential written into the
descriptor — which is versioned script content. Both this map and the script's
own environment variables apply to the deploy-time parse as well as the run, so
an `env_var()` feeding a schema, alias or `enabled` produces the same relation
in the stored graph and in the build either way. Prefer the descriptor's `env`
when the value belongs to the project rather than to one deployment of it: it is
versioned with the descriptor, so a redeploy from git carries it.

`select`/`exclude`/`selector` are passed **verbatim** to dbt. Do not reimplement
the selector grammar; Cosmos's manifest path had to, and it is a recurring source
of divergence. `select` and `vars` are overridable per run via job args. One boundary to know:
the **cascade follows the deployed descriptor**, not the run — for both of them. Asset rows are
written at deploy like every other language's, so narrowing `select` for an
ad-hoc run still notifies consumers of the models it skipped, and widening it
does not notify consumers of the extra ones. Treat the run-arg overrides as what
they are — an ad-hoc scope — and split the project into several scripts
(decision 6) when the *graph* should differ.

A `vars` override is the same: gating the graph refresh on it would leave that
override's relations recorded for the next default run, which then builds the
descriptor's and dispatches from the override's. What DOES refresh per run is a
property of the descriptor — a `{{ }}` placeholder in `vars` or a `$var:` value
in `env` — because those are dynamic on every run, not just the one that passed
an argument.

`vars` interpolates from job args with `interpolate_template` (`common.rs`,
shared with the Ansible executor). The syntax is `{{ arg_name }}`.

`select`/`exclude` also scope **what the script owns in the graph**, resolved by
asking dbt (`dbt ls --output json`) rather than by interpreting the selector
string. Without that a narrowly-selected script registers as the producer of
every model in the project and its cascade fires downstream of models it never
builds. Running several scripts with different selections
only composes because of this.

## Deploy path

New `ScriptLang::Dbt` arm in `worker_lockfiles.rs` (near the `ScriptLang::Ansible`
arm at :2758), producing:

```rust
struct DbtDependencyLocks {
    manifest_digest: String,
    engine: String,
    engine_version: String,
}
```

Steps: write the script's modules into the job directory, `dbt deps`, `dbt parse`
for the manifest, then ingest.

Ingestion writes the rows the native parser writes, via
`replace_static_asset_usage` (`windmill-common/src/assets.rs:254`) into
`asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)`.
The language dispatch point is `parse_assets_for_lang`
(`windmill-api-scripts/src/asset_inference.rs:33`).

**The one architectural wrinkle.** Every other language's asset parsing there is a
pure function of script content. dbt's needs the bundle on disk and a dbt
invocation, so it cannot run inline: it runs as a deploy-time job, persists the manifest, and
`parse_assets_for_lang` reads the persisted result. **Prototype this first**, it
is the assumption most likely to reshape the phasing.

## Run path

One `dbt build` per job, the shape Cosmos arrived at with `ExecutionMode.WATCHER`
after per-model Airflow tasks proved roughly 6x slower (about 5.5 minutes for one
`dbt run` versus about 32 minutes for 184 per-model invocations on
google/fhir-dbt-analytics). dbt's own threading provides parallelism; Windmill
provides observability.

1. Materialise the script's modules into the job directory, restore
   `dbt_packages/` from cache.
2. Render `profiles.yml` from the resource, or use the project's own file with
   Windmill secrets injected as env vars for `{{ env_var() }}`.
3. `dbt build --log-format json` plus `select`/`exclude`/`vars`/`threads`.
4. Stream events: each `NodeFinished` updates per-model status live and emits
   `RecordMaterializationRequest` (`windmill-common/src/materialization.rs:53`),
   which already carries `asset_kind`, `asset_path`, `partition`, `status`,
   `row_count`, `job_id`, `error`, `schema`. `run_results.json` supplies all of it.
5. Structured job result (per-model status, timing, rows, failed tests), not just
   an exit code. Partial failure is dbt's normal case and must be legible without
   reading logs.
6. **Node-level retry.** `retry_failed_nodes: {attempts, delay_seconds}` in the
   descriptor rebuilds only what a failed build left failed or skipped, in the
   same job, before reporting failure. dbt confines a failure to its own
   subtree, so a transient warehouse error costs those nodes rather than the
   project. In-job is what keeps the state question out of it: the previous
   attempt's `run_results.json` is still in the job directory, so there is
   nothing to persist and no worker to land back on. This is the granularity
   astronomer-cosmos gets from one Airflow task per model, without the ~6x that
   per-model tasks measured (decision 4).

   A retry's `run_results.json` names only the nodes it redid, so it overlays
   the accumulated results rather than replacing them: the job's result must be
   every node the job touched, or the nodes that succeeded before the retry
   settle no materializations.

7. `dbt retry` resumes from the failure point using `run_results.json`, which is
   what makes one-job-per-invocation defensible. It is saved twice: to the
   worker's local cache, and to `dbt_run_state` in the database, so a retry
   works from any worker of the group. Only `run_results.json` is stored there.
   `dbt retry` also needs `manifest.json`, roughly sixty times larger and
   growing with the project (732 KB against 12 KB on a six-node fixture), but
   the manifest is a pure function of the project files, vars and env — all of
   which the stored identity already pins — so a worker restoring from the
   database re-derives it with a `dbt parse` of about a second. It is a run argument
   (`dbt_command: retry`) rather than the automatic behavior of Windmill's
   generic retry, which has no per-language hook to change the invoked command.
   Each attempt gets a fresh job dir, so the previous run's `target/` is cached
   per (workspace, script) on the worker and restored for a retry.
8. Test failures honor dbt's `severity`: `error` fails the job, `warn` surfaces
   without failing. Overriding this would make the same project behave differently
   on Windmill than locally, breaking the core promise.

## Two decisions the implementation narrowed

**Decision 13 — no S3 copy of the manifest.** The sidecar holds every field the
graph renders; nothing reads a stored `manifest.json`, so writing one to S3
would be an unread copy of data that is already reproducible by redeploying (or,
for a dynamic descriptor, by the next run). Worth adding the day something needs the
parts the sidecar drops — compiled SQL, macro definitions — and not before.

**Decision 14 — column lineage is not available.** The decision assumed
`manifest.json` carries column-to-column edges; it does not, in either core
engine. What it does carry is declared column *descriptions*, which are
ingested. Real column lineage would need Fusion (which does static analysis) or
a SQL-AST pass of our own, so `columnLineageGraph.ts` is not wired up for dbt.

## Concept mapping

| dbt | Windmill | Mechanism |
|---|---|---|
| model relation | `table://` asset | new `AssetKind` |
| `ref()` graph | lineage edges | `replace_static_asset_usage` |
| `materialized: table` | `materialize_strategy: replace` | `AssetGraphRunnableNode` |
| `materialized: incremental` | `append` or `merge` (by `unique_key`) | same |
| `{% snapshot %}` | `scd2` | same, incl. `<dim>_current` handling |
| `unique`/`not_null`/`accepted_values`/`relationships` | `data_tests` | exact 1:1 with the four `// data_test` kinds |
| declared column metadata | `columns` on the asset node | descriptions only; see the note below |
| model `tags` | node badge | `tag` |
| source freshness | `freshness` | `last_success_at` chip |
| `run_results.json` | materialization records | `record_materialization` |
| `dbt_packages/` | worker-local cache | keyed by `packages.yml` and the project digest |

## Phases

**Phase 1: run it.** `ScriptLang::Dbt` across the 41 `ADD_NEW_LANG` sites (mostly
one-liners in `EditorBar.svelte`, `scripts.ts`, `script_helpers.ts`,
`LanguageIcon.svelte`, `script_common.ts`). Engine provisioning for all three
options in `Dockerfile` and `docker/DockerfileFull*` (bundle 1x and 2x, fetch
Fusion at runtime). New `backend/windmill-worker/src/dbt_executor.rs`: descriptor
parse, bundle materialisation, `profiles.yml` render, `dbt build`, log
passthrough, structured result, retry.

**Phase 2: graph.** `DbtDependencyLocks` and the deploy arm. Migration via
`cargo sqlx migrate add -r dbt_manifest_sidecar`. New `table://` `AssetKind` with
its `canonical_prefix`. Manifest ingest. Deploy-time ingest plus the per-run
re-ingest for dynamic descriptors. Extend `AssetGraphRunnableNode`/`AssetGraphAssetNode` in
`frontend/src/lib/components/assets/AssetGraph/types.ts` with dbt provenance and
render through the existing `RunnableNode.svelte` / `AssetNode.svelte` /
`DataTestNode.svelte`.

**Phase 3: live progress and ergonomics.** JSON event stream to per-model status
on the canvas mid-run. `record_materialization` per model. Profile and select
pickers in the editor. Per-model failure triage in the run view.

**Phase 4 (not in this PR).** `--defer` and `state:modified`. Partition and
backfill integration so `BackfillRangeDialog.svelte` works on dbt models.
`wmill dbt import <dag.py>` reading `DbtDag(...)` kwargs.

## E2E test requirements

Against a real dbt project (jaffle_shop shape) and the local Postgres:

1. **Happy path**: deploy a dbt script, run it, assert models exist in the
   warehouse and the job succeeds with a structured per-model result.
2. **Engine parity**: the same project passes on `dbt-core-1x` and `dbt-core-2x`.
   Fusion covered by a manually-run test, not CI (runtime fetch).
3. **Test severity**: a failing `error`-severity test fails the job; a failing
   `warn`-severity test does not.
4. **Retry**: a run failing midway, retried, resumes via `dbt retry` and does not
   rebuild already-successful models.
5. **Graph ingest**: after deploy, model assets and `ref()` edges exist; a native
   script reading one of the marts gets an edge to it.
6. **Cross-boundary cascade**: a dbt mart write triggers a downstream native
   pipeline script declaring a read on it.
7. **Selection**: descriptor `select`/`exclude`, and a run-arg override, each
   build only the expected subset. `dbt_command` offers `build` and `retry`
   only: a command building a subset of the registered writes would notify
   consumers of relations it left stale.
8. **Dynamic descriptors**: a `{{ }}` placeholder in `vars` re-ingests the graph
   from the run's own manifest, so a model that placeholder enables appears in
   the same run that builds it.
9. **Both credential paths**: resource-rendered `profiles.yml`, and the project's
   own `profiles.yml` with env-var injection.
10. **Caching**: a second run reuses the cached `dbt_packages/` with no network
    fetch.

Keep only tests that pin behavior a future change could break. Per AGENTS.md,
delete development scaffolding before marking the PR ready.
