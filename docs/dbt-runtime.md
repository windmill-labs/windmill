# Windmill as a dbt runtime

Implementation spec for running an existing dbt project on Windmill with no
changes to the project itself. Companion to [`pipelines-vs-dbt.md`](./pipelines-vs-dbt.md),
which covers the opposite direction (native pipeline features that replace dbt).
The two are complementary: this is the adoption ramp, that is the long game.

Benchmark to beat is Airflow + [astronomer-cosmos](https://astronomer.github.io/astronomer-cosmos/),
the dominant way dbt is orchestrated today.

## Scope

- **In**: run an unmodified dbt project from a git repo, one Windmill job per
  invocation, live per-model observability, dbt models as first-class assets in
  the existing asset graph.
- **Out**: one Windmill job per dbt model, `state:modified` / slim CI,
  `dbt docs` hosting, semantic layer, dbt platform integration.
- **CE**: the runtime, the manifest ingest, the asset graph and every piece of
  UI ship in CE, as do all adapters except two. Only the `mssql` and `oracle`
  adapters are EE, mirroring the native `ScriptLang` boundary (decision 21), and
  private-repo GitHub App auth is EE inherited from git-sync, not introduced
  here.

## Decision log

| # | Decision | Resolution |
|---|---|---|
| 1 | dbt engine | Three-way toggle (`dbt-core-1x` \| `dbt-core-2x` \| `fusion`); shipped default `dbt-core-1x`, instance-configurable. See below |
| 2 | Artifact shape | `ScriptLang::Dbt` |
| 3 | Graph in v0 | Yes, both runtime and graph |
| 4 | Execution granularity | One job per invocation |
| 5 | Ref pinning | Both pinned and `latest`, pinned by default |
| 6 | Multiple run configs | N scripts against the same repo |
| 7 | Run-time `select` | Descriptor default plus run-arg override |
| 8 | Credentials | Both `profiles.yml` passthrough and resource mapping |
| 9 | Adapter mappings | postgres, redshift, mysql, snowflake, bigquery, databricks; others via the project's own `profiles.yml` |
| 10 | Private repo auth | SSH/token in CE; GitHub App **not yet** — see below |
| 11 | Asset kind | `table://<resource>/<schema>/<name>`, not `dbt://`. See below |
| 12 | Graph refresh | Falls out of #5, no separate mechanism. See below |
| 13 | Manifest storage | Sidecar table for nodes/edges, full manifest to S3 |
| 14 | Metadata depth | Full (tests, column lineage, strategy, tags, freshness) |
| 15 | Node rendering | Asset nodes per model plus one runnable node for the script |
| 16 | Progress | Live, from the JSON event stream |
| 17 | Test failures | Honor dbt's own `severity` |
| 18 | Retry | `dbt retry` by default, full re-run available |
| 19 | Caching | Worker-local global cache, keyed by digest and commit |
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

## Decision 12: refresh falls out of ref strategy

The lockfile does most of the work, and pairing it with a `latest` strategy
removes the need for a separate refresh mechanism. Two modes, no manual button
and no webhook in v0:

- **Pinned** (default). Deploy resolves `ref` to a full commit hash and stores it
  in the lockfile alongside the parsed graph. Runs are reproducible and the graph
  always matches what executes. "Refresh" is just "redeploy", which re-resolves
  and re-parses. No new concept.
A `ref` or a `vars` value spelled with a `{{ placeholder }}` behaves like
`latest` for graph purposes: the deploy cannot know what will run, and dbt vars
can steer `enabled`, aliases, schemas, databases and materializations, so the
graph is re-ingested from every run's own manifest. Since asset dispatch fans out
from the stored rows, a run that cannot refresh them fails rather than cascading
from a stale graph — which also means those descriptors cannot run on an agent
worker, whose only DB access is through the API.

Second boundary to accept: those rows are keyed by script path, so **two
concurrent runs of one dynamic script race**. Each replaces the other's rows
before dispatch reads them, and either job can end up notifying the other's
consumers. Give such a script a concurrency limit of 1. Fixing it properly means
dispatching from a per-job snapshot of what the run actually wrote, which is a
change to the shared cascade rather than to dbt.

- **`ref: latest`.** Resolve HEAD at run time. The graph must then refresh every
  run or it diverges from execution, and that is nearly free: the run already
  invokes dbt, which already writes `target/manifest.json`, so ingest that
  instead of paying for a separate `dbt parse`.

One wrinkle to accept and surface in the UI: under `latest`, a newly added model
has no graph entry until its first run completes, so nodes appear one run late.
Strictly better than a stale graph presented as authoritative.

## Where the dbt project lives

An external git repo, referenced by a `git_repository` resource, cloned onto the
worker at job time. Not in the Windmill workspace, not in the `wmill sync` tree.
The Windmill script is a pointer plus run configuration.

Storing the project in Windmill was rejected: scripts have no multi-file
representation, so it means inventing one plus a migration path, for no user
benefit. If a workspace's git-sync repo also holds the dbt project in a
subdirectory, point the resource at that repo; same mechanism, no extra code.
Document that as a recommended layout.

Consequences: **Windmill does not version the models, the pinned commit does**,
and model changes need a repo push plus a redeploy (pinned) or just the next run
(`latest`), never a script edit.

## The script artifact

New `ScriptLang::Dbt`. Content is a YAML descriptor whose field names track dbt's
and Cosmos's vocabulary so the mental model ports without translation:

```yaml
repo: $res:u/rf/analytics_repo    # git_repository resource
project: transform                # subdir containing dbt_project.yml
ref: v2.3.0                       # tag/branch/commit, or `latest`
engine: dbt-core-1x               # or dbt-core-2x | fusion
profile:
  resource: $res:f/prod/snowflake # rendered into profiles.yml
  target: prod
  # schema: marts                 # target schema; REQUIRED for BigQuery, whose
                                  # resource is a service-account JSON with no
                                  # dataset in it
  # profiles_yml: profiles.yml    # alternative: keep your own file
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
git_ssh_identity: []              # variables holding private keys, for SSH remotes
```

`env` values spelled `$var:<path>` are resolved to that Windmill variable, so a
project keeping its own `profiles.yml` never needs a credential written into the
descriptor — which is versioned script content. Private repos authenticate two ways: a
token in the resource's URL, and `git_ssh_identity` for SSH remotes.

**GitHub App resources are rejected, with that reason.** Decision 10 assumed the
support was inherited from git-sync, and it is not: the only helper that mints an
installation token (`get_github_app_token_internal`) authorizes by matching the
job against the workspace's *configured git-sync scripts*, so it refuses a dbt
job outright. Minting for an arbitrary runnable needs its own authorization path.
Until that exists the descriptor fails with a message naming the two paths that
do work — a stated limitation beats a clone failing on an auth error the user
cannot connect to a cause.

`select`/`exclude`/`selector` are passed **verbatim** to dbt. Do not reimplement
the selector grammar; Cosmos's manifest path had to, and it is a recurring source
of divergence. `select` and `vars` are overridable per run via job args. One boundary to know:
the **cascade follows the deployed descriptor**, not the run. Asset rows are
written at deploy like every other language's, so narrowing `select` for an
ad-hoc run still notifies consumers of the models it skipped, and widening it
does not notify consumers of the extra ones. Treat the run-arg override as what
it is — an ad-hoc scope — and split the project into several scripts
(decision 6) when the *graph* should differ.

`vars` and `ref` interpolate from job args with `interpolate_template`
(`common.rs`, shared with the Ansible executor, which is how Ansible already
parameterizes commits). The syntax is `{{ arg_name }}`.

`select`/`exclude` also scope **what the script owns in the graph**, resolved by
asking dbt (`dbt ls --output json`) rather than by interpreting the selector
string. Without that a narrowly-selected script registers as the producer of
every model in the project and its cascade fires downstream of models it never
builds. Running several scripts against one repo with different selections
(decision 6) only composes because of this.

## Deploy path

New `ScriptLang::Dbt` arm in `worker_lockfiles.rs` (near the `ScriptLang::Ansible`
arm at :2758), producing:

```rust
struct DbtDependencyLocks {
    repo_url: String,
    commit: String,          // full hash; empty when ref == latest
    manifest_digest: String,
    engine: String,
    engine_version: String,
}
```

Steps: resolve commit (`get_git_repo_full_head_commit_hash`), shallow clone
(`clone_repo_without_history`, `ansible_executor.rs:474`), `dbt deps`, `dbt parse`
for `target/manifest.json`, then ingest.

Ingestion writes the rows the native parser writes, via
`replace_static_asset_usage` (`windmill-common/src/assets.rs:254`) into
`asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)`.
The language dispatch point is `parse_assets_for_lang`
(`windmill-api-scripts/src/asset_inference.rs:33`).

**The one architectural wrinkle.** Every other language's asset parsing there is a
pure function of script content. dbt's needs a clone and a dbt invocation, so it
cannot run inline: it runs as a deploy-time job, persists the manifest, and
`parse_assets_for_lang` reads the persisted result. **Prototype this first**, it
is the assumption most likely to reshape the phasing.

## Run path

One `dbt build` per job, the shape Cosmos arrived at with `ExecutionMode.WATCHER`
after per-model Airflow tasks proved roughly 6x slower (about 5.5 minutes for one
`dbt run` versus about 32 minutes for 184 per-model invocations on
google/fhir-dbt-analytics). dbt's own threading provides parallelism; Windmill
provides observability.

1. Clone at the pinned commit (cached, commits are immutable), restore
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
6. `dbt retry` resumes from the failure point using `run_results.json`, which is
   what makes one-job-per-invocation defensible. It is a run argument
   (`dbt_command: retry`) rather than the automatic behavior of Windmill's
   generic retry, which has no per-language hook to change the invoked command.
   Each attempt gets a fresh job dir, so the previous run's `target/` is cached
   per (workspace, script) on the worker and restored for a retry.
7. Test failures honor dbt's `severity`: `error` fails the job, `warn` surfaces
   without failing. Overriding this would make the same project behave differently
   on Windmill than locally, breaking the core promise.

## Concept mapping

| dbt | Windmill | Mechanism |
|---|---|---|
| model relation | `table://` asset | new `AssetKind` |
| `ref()` graph | lineage edges | `replace_static_asset_usage` |
| `materialized: table` | `materialize_strategy: replace` | `AssetGraphRunnableNode` |
| `materialized: incremental` | `append` or `merge` (by `unique_key`) | same |
| `{% snapshot %}` | `scd2` | same, incl. `<dim>_current` handling |
| `unique`/`not_null`/`accepted_values`/`relationships` | `data_tests` | exact 1:1 with the four `// data_test` kinds |
| column-level lineage | `column_lineage` | `columnLineageGraph.ts`, `ColumnLineageTrace.svelte` |
| model `tags` | node badge | `tag` |
| source freshness | `freshness` | `last_success_at` chip |
| `run_results.json` | materialization records | `record_materialization` |
| `dbt_packages/` | worker-local cache | keyed by `packages.yml` digest |

## Phases

**Phase 1: run it.** `ScriptLang::Dbt` across the 41 `ADD_NEW_LANG` sites (mostly
one-liners in `EditorBar.svelte`, `scripts.ts`, `script_helpers.ts`,
`LanguageIcon.svelte`, `script_common.ts`). Engine provisioning for all three
options in `Dockerfile` and `docker/DockerfileFull*` (bundle 1x and 2x, fetch
Fusion at runtime). Extract `clone_repo`, `clone_repo_without_history`,
`get_git_repo_full_head_commit_hash` from `ansible_executor.rs` into a shared
`git_clone.rs`; they are ansible-agnostic already. New
`backend/windmill-worker/src/dbt_executor.rs`: descriptor parse, clone,
`profiles.yml` render, `dbt build`, log passthrough, structured result, retry.

**Phase 2: graph.** `DbtDependencyLocks` and the deploy arm. Migration via
`cargo sqlx migrate add -r dbt_manifest_sidecar`. New `table://` `AssetKind` with
its `canonical_prefix`. Manifest ingest. Both ref strategies with their refresh
behavior. Extend `AssetGraphRunnableNode`/`AssetGraphAssetNode` in
`frontend/src/lib/components/assets/AssetGraph/types.ts` with dbt provenance and
render through the existing `RunnableNode.svelte` / `AssetNode.svelte` /
`DataTestNode.svelte`.

**Phase 3: live progress and ergonomics.** JSON event stream to per-model status
on the canvas mid-run. `record_materialization` per model. Repo/profile/select
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
   build only the expected subset.
8. **Ref strategies**: pinned runs the locked commit after the branch advances;
   `latest` picks up the new commit and refreshes the graph from the run's own
   manifest.
9. **Both credential paths**: resource-rendered `profiles.yml`, and the project's
   own `profiles.yml` with env-var injection.
10. **Caching**: a second run reuses the cached clone and `dbt_packages/` with no
    network fetch.

Keep only tests that pin behavior a future change could break. Per AGENTS.md,
delete development scaffolding before marking the PR ready.
