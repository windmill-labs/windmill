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
| 8 | Credentials | Workspace warehouses, plus `profiles.yml` passthrough. A descriptor never names a resource. See below |
| 9 | Adapter mappings | postgres, redshift, mysql, snowflake, bigquery, databricks translate from their Windmill resource; **every** adapter dbt has is reachable from a `dbt_profile` resource, or the project's own `profiles.yml` |
| 10 | Private repo auth | Not applicable: the project is synced, not fetched |
| 11 | Asset kind | `dbt://<warehouse>/<schema>/<name>` — keyed on the relation, not on dbt's node id. See below |
| 12 | Graph refresh | Deploy-time, re-ingested per run only when the descriptor is dynamic, plus an explicit `parse` of the editor's buffer. See below |
| 13 | Manifest storage | Sidecar table for nodes/edges. Full manifest **not** stored — see below |
| 14 | Metadata depth | Tests, strategy, tags, freshness, column descriptions. Column **lineage** is not in the manifest — see below |
| 15 | Node rendering | Asset nodes per model plus one runnable node for the script |
| 16 | Progress | Live, from the JSON event stream |
| 17 | Test failures | Honor dbt's own `severity` |
| 18 | Retry | Automatic node-level retry in-job, plus `dbt retry` as a run argument. See below |
| 19 | Caching | Worker-local global cache, keyed by the project digest and the resolution the deploy pinned |
| 20 | Images | Full images only |
| 21 | Licensing | CE except the `mssql` / `oracle` adapters. See below |
| 22 | Naming | Match Cosmos field names; importer deferred |
| 23 | Descriptor | `wm_dbt.yaml` inside the project, OPTIONAL. See below |
| 24 | Warehouse | Configured on the workspace by name, `main` by default. See below |

## Decision 1: engine toggle, and why the shipped default is not Fusion yet

`engine: dbt-core-1x | dbt-core-2x | fusion` in the descriptor. Omitted, it is
`dbt-core-1x`, which runs today's projects untouched.

No engine is baked into any image. Each is fetched or built on first use and
cached, for a different reason in each case.

| Engine | Distribution | Cold start | License |
|---|---|---|---|
| `dbt-core-1x` (default) | A uv venv resolved per adapter on first use, then cached. **Cannot** be baked: the adapter is a Python package chosen per project | One venv build per (core range, adapter) | Apache 2.0 |
| `dbt-core-2x` | One adapter-agnostic Rust binary, fetched from GitHub releases on first use, cached | One download | Apache 2.0 |
| `fusion` | **Never bundled.** Fetched from dbt Labs on first use, cached | One download (~290MB) | dbt Fusion engine license agreement |

2.x is the one that *could* be baked, and deliberately is not: it is a
pre-release (`2.0.0-alpha.5`) that nothing is defaulted onto, so baking it costs
a layer in every image and a version pinned in two places with nothing keeping
them in step. An operator who wants an engine pre-staged — an air-gapped
instance, or a fleet that should not fetch per worker — populates
`DBT_BUNDLED_DIR` (default `/usr/local/dbt`) with `core2x-<version>/dbt-sa-cli`
in a derived image; the worker prefers it over its own cache.

Two things to know before choosing 2.x: it is a pre-release, and it does not
emit the per-node events the run page animates (see "Live per-model progress"),
so a run on it reports its models only at the end.

The 1.x venv resolves `dbt-core>=1.8,<2.0.0` *together with* the adapter rather
than pinning a core version, because several adapters cap below the newest core
(dbt-oracle and dbt-databricks below 1.12) and an independent pin makes those
projects unprovisionable. The lockfile records whichever version the resolver
actually chose. Both bounds and each engine version are env-overridable
(`DBT_CORE_1X_FLOOR`, `DBT_CORE_1X_CEILING`, `DBT_CORE_2X_VERSION`).

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
and a real project is verified end to end on it. Both dbt-core engines are
exercised by the e2e suite, so the flip is a config change, not a port.

## Decision 21: mirror the native warehouse boundary, do not invent one

Everything structural is CE: the executor, all three engines, the manifest
ingest, the `dbt://` asset graph, live progress, the editor. The only gate is
on two adapters, and it is not a dbt-specific policy — it is the same boundary
the native script languages already draw. Since `bigquery` and `snowflake`
became CE, the only warehouse `ScriptLang`s still behind a license are `mssql`
and `oracledb`, so those two dbt adapters are EE and every other one (postgres,
mysql, duckdb, snowflake, bigquery, databricks, redshift, clickhouse,
salesforce) is CE. Gating any of the others would make reaching a warehouse
through dbt stricter than reaching it natively, which is backwards.

Those two are *recognized* (for the gate and for the pip package the 1.x
engine's venv needs), but no Windmill connection resource translates into them:
an `oracledb` resource is `{user, password, database}` with no
host/protocol/service, and dbt-sqlserver needs an ODBC `driver` the images do not
install. They reach their warehouse through a `dbt_profile` resource or the
project's own `profiles.yml`, which is also how duckdb, clickhouse and salesforce
work.

Recognition is what the gate keys on, and it survives the open adapter set: a
`dbt_profile` stating `sqlserver`, `mssql` or `oracle` resolves to the same
`KnownAdapter` a resource type would, so it is gated identically. An adapter
Windmill has never heard of is never enterprise — the boundary mirrors the two
native warehouse languages, and an adapter with no Windmill runtime behind it is
not one of them.

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

## Decision 11: `dbt://`, keyed on the relation and not on the dbt node

`dbt://<warehouse>/<schema>/<name>`, one `AssetKind`, where `<warehouse>` is the
workspace warehouse's NAME, so two scripts running against the same warehouse
agree on identity.

The SCHEME names the producer, because dbt is the only thing that creates one of
these: no other language derives warehouse relations, `// materialize` takes
DuckLake targets only, and a dbt run does not dispatch. Calling the kind
something generic promised a parity with native Snowflake and BigQuery scripts
that does not exist.

The PATH is the physical relation, and that is the load-bearing half. dbt-core
has no cross-project `ref()`: two projects meet when one materializes a mart and
the next declares it a `source`. Their dbt identities differ there —
`model.a_pkg.orders` against `source.b_pkg.analytics.orders` — while the relation
does not, so keying on `unique_id` would make every project an island and turn
the handoff into two unconnected nodes. `unique_id` also embeds the package name
from `dbt_project.yml`, which two unrelated projects may both call `analytics`,
collapsing two different tables onto one node. The relation cannot collide that
way. It is also what a DuckDB, Python, TS or Ansible script can name in a
`// on dbt://…` annotation to join the lineage — those four are the languages
with a body-asset parser; the native SQL ones cannot declare assets at all.

A dbt run does **not** trigger those readers. See "no cascade from dbt" below.

An ephemeral model (an inlined CTE, never written), an exposure, or a source that
is not separately modelled has no physical relation and therefore no place in
this namespace. If those ever prove worth rendering they need a key of their own
— `unique_id` suits them, precisely because nothing else can refer to them.

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
use. The warehouse-name prefix is spelled as the workspace configures it and
stays case-sensitive.

**Warehouse identity is the workspace warehouse's name**, exactly as
`ducklake://main.orders` keys on the workspace lake's name — never the host,
account or database. A descriptor cannot name a resource at all (Decision 24), so
there is exactly one spelling per warehouse and the ambiguity a per-project
resource would create does not arise. The warehouse names the default database
too, so it stays out of the key; a model that *overrides*
its database (Snowflake `database`, BigQuery `project`) is genuinely elsewhere and
qualifies its schema segment as `<database>.<schema>`, so two same-named relations
in different databases cannot collapse onto one node. A project that brings its own
`profiles.yml` reports its target's database from that file, read with the same
keys the renderer writes, so it spells a relation exactly as a workspace-warehouse
project does and the two meet on one node. Only where the target leaves its
database implicit does every relation qualify, because assuming they share one
database is exactly what would collapse them.

Three call sites derive this key: the manifest ingest that creates the node, and
the live-progress and end-of-run paths that record status against it. They share
one function, because a site that derives it differently records progress against
a path no node has — the run still succeeds and the graph simply never moves. The same
warehouse is reachable under several hostnames, and credential material has no
business in an asset key. Accepted limitation, worth knowing before it is
filed as a bug: **two workspace warehouses pointing at the same physical
warehouse do not unify**, so assets under one will not share edges with assets
under the other. Point both projects at one warehouse to link them.

## Decision 24: the warehouse is a workspace setting, named, and the only one

A descriptor names a warehouse by NAME (`profile.warehouse`, `main` when it names
none) and cannot name a resource. Admins configure the warehouses under Settings
→ dbt, where each entry points at a resource, exactly as `large_file_storage`
points at the object-storage resource and a DuckLake names its catalog.

**What a warehouse may point at.** Either a Windmill connection resource whose
type `render_profile` translates (`postgresql`, `redshift`, `mysql`, `snowflake`,
`snowflake_oauth`, `bigquery`, `gcp_service_account`, `databricks`), or a
**`dbt_profile`** resource, whose VALUE IS one entry of that file's `outputs`
map — `type` included, nothing lifted out or renamed. A block is copied from a
working `profiles.yml` and pasted in, which is the whole point: a type that asked
the user to restructure their block first would be doing the translation this
exists to avoid. Its schema declares no properties, so the resource form renders
one JSON editor over the value (`ResourceForm.svelte`). The picker is
constrained to exactly these (`WAREHOUSE_RESOURCE_TYPES`); anything else has no
way to become a target at all, which is why an unconstrained picker was a trap:
it offered slack and github resources for a field that can only be a warehouse.

The two exist for different reasons. A Windmill resource is the ergonomic path
and is shared with everything else that connects to that warehouse, but it is
*not* a dbt target: each adapter arm translates the fields Windmill's resource
happens to carry into the keys dbt reads, so only what an arm covers can be
expressed, and an adapter with no arm cannot be reached from one at all.
`dbt_profile` inverts that — nothing is translated, so any adapter and any key it
documents works.

Which of the two a value is cannot be read off the value: both are objects with a
`type`, and Windmill's bigquery resource is a service-account JSON that says
`type: service_account`. So the warehouse carries its resource's TYPE
(`DbtWarehouseConnection.resource_type`), and that is also what finally makes
decision 9's "the resource type name is the authority" true at runtime rather
than aspirational — the translated path resolved its adapter by sniffing
connection fields until it had the name.

**`dbt_profile` is open, deliberately.** Its `type` is not checked against a list:
`DbtAdapter` carries an optional `KnownAdapter` beside the name, so the eleven
adapters Windmill has facts about (a field mapping, a pip package, the license
gate) keep them, and every other adapter dbt has — `trino`, `athena`, `spark`,
whatever ships next — is carried by name and rendered, licensed and identified
without Windmill knowing anything about it. A closed list would have made
"whatever dbt supports" mean "whatever this enum lists", and each new adapter a
Windmill release. The name is constrained to `[a-z0-9_-]` starting alphanumeric
*because* it is open: it reaches a pip requirement and a venv path on the host,
where a leading `-` is a flag and a `/` is a path segment.

**Installing one is a separate question from using one.** `dbt-core-1x` fetches
`dbt-<name>` from PyPI, `dbt-` is not a reserved prefix there, and that install
runs through `run_tool` — outside the nsjail ordinary Python dependency
installation uses, with uv executing a source distribution's PEP 517 backend. An
unbounded name would therefore let a script author publish `dbt-<x>` and run code
as the worker, on the one dependency path that is not sandboxed. So
`ensure_adapter_installable` gates that install on `PUBLISHED_ADAPTERS` plus
whatever an operator lists in `DBT_EXTRA_ADAPTERS`: the author chooses which
adapter to use, the admin decides which packages this instance trusts. Nothing
else is gated — a profile still renders for any adapter, and `dbt-core-2x` and
`fusion` carry their adapters in the binary, install nothing, and take any
`type` at all.

Two keys are not passed through: `type` (Windmill writes the adapter's own dbt
spelling) and `root_certificate_pem`, which is a PEM body rather than the path
dbt hands the driver — it is written beside `profiles.yml` and pointed at by
`sslrootcert`, as it is for a translated postgres resource. `profile.schema` and
`threads` from the descriptor override their block keys rather than joining them.

Three things follow, and they are the reason for the rule rather than
consequences to work around.

**A dbt project carries no connection.** The same project runs locally against a
developer's own `~/.dbt/profiles.yml` and on Windmill against the workspace
warehouse, with no Windmill-specific file in between and nothing to strip before
committing it to a repository. This is what makes Decision 23 possible at all: if
a project had to name its own resource, the descriptor could never be optional.

**Asset identity has exactly one spelling.** Keying on a name is only sound
because a name is all there is. Had both `profile.resource` and
`profile.warehouse` existed, one physical warehouse would be reachable under two
spellings and two projects on it would silently fail to share nodes — the exact
failure Decision 11 exists to prevent.

**dbt is unpermissioned, and the blast radius is bounded by construction
instead.** The warehouse resource is read with NO permission check on the runner,
exactly as `s3://` reaches the workspace bucket without the caller being granted
the storage resource: configuring a warehouse is what makes it available, and
anyone who may run a dbt script may build with it and read its models. What is
reachable stays bounded because only an admin writes the setting and a descriptor
cannot name a resource, only one of the names an admin configured.

Per-relation rules were considered and rejected. `s3://` can enforce a path glob
because Windmill mediates every object operation through its proxy; dbt has no
such chokepoint — Windmill renders `profiles.yml` and dbt opens its own
connection. A rule could only be a pre-run check against the manifest, and a
`pre-hook`, a macro or `dbt run-operation` issues arbitrary SQL on the same
connection, so it would stop the ordinary case while implying a guarantee it
cannot keep.

A project that brings its own `profiles.yml` still connects with it, and then
names a warehouse only to say where its assets belong. The name must still match
a configured warehouse — a typo is not identity, it strands the project's models
on a node nothing else reaches — but it grants nothing, since nothing here is
granted. It gets no identity by default, because defaulting to `main` would key a
self-hosted profile's tables onto a workspace warehouse it never connected to.

That label is worth having only because such a project spells its relations the
same way: Windmill reads the target's database out of the project's own file
(Decision 11), so a mart it builds and a workspace-warehouse project's `source`
on the same relation land on ONE node. Without that the label would name a
namespace and still share nothing, which is the failure it exists to prevent.

An agent worker cannot read the database, so it resolves the name through a
job-scoped API route. That route returns the resolved connection, which is why it
requires a job token: a running job already holds those credentials in its
rendered `profiles.yml`, and a browsable route would hand them to anyone. The
same worker posts its per-model outcomes to a second job-scoped route, since the
live reporter tails a log straight into the database and cannot run there. An
agent's run page therefore fills in when the run ends rather than during it.
Both routes are posted with the JOB's token: an agent's own credential
authenticates only against the agent surface.

## Decision 23: the descriptor is optional, and lives inside the project

`<script>__dbt/wm_dbt.yaml`. An unmodified dbt project — one `cp -r` away from a
developer's working copy, or a repository cloned as-is — is already a complete
Windmill script: it runs the whole project against the workspace's default
warehouse. The descriptor appears only when the project wants something
Windmill-specific: run arguments, a named warehouse, an engine pin, a test
policy.

It lives INSIDE the project rather than beside it so that an author writes
nothing outside the directory dbt itself reads. A dbt developer's working copy
and a Windmill bundle are then the same directory, which is the whole bargain of
Decision 5.

Absent means an empty descriptor, never a missing script. `dbt_project.yml` is
what identifies a project — the descriptor cannot, being optional — and three
rules keep "absent" from reading as a change: the export omits an empty
descriptor, the sync map gives BOTH sides the empty descriptor an absence means
(so neither reads as an addition), and a pull deletes rather than writes one.
Without all three a descriptor-less project either diffs forever or grows the
very file this decision exists to avoid.

## Decision 12: the graph refreshes with the deploy

The project's files are the script's, so a deploy already sees exactly what will
run: it parses the bundle and stores the graph. "Refresh" is just "redeploy". No
manual button, no webhook, no separate mechanism.

The one case that cannot be settled at deploy is a descriptor that is dynamic by
construction: a `vars` value spelled with a `{{ placeholder }}`, or an `env`
value spelled `$var:` (re-resolved every run). dbt vars can steer `enabled`,
aliases, schemas, databases and materializations, so for those the deploy cannot
know what will run and the graph is re-ingested from every run's own manifest,
under that run's job id. A run that cannot refresh those rows fails rather than
showing a stale graph. What the SCRIPT owns stays the deploy's — see "Which
run's graph becomes what the script owns" for why the two cannot diverge.

An agent worker reaches the database only through the API, so it POSTs the graph
it parsed to `/api/agent_workers/dbt_graph/{workspace}` instead of writing it —
which is why it needs no way to READ the stored relation root: it re-ingests
every run, so its own run page shows the profile it actually used. What it
publishes is that per-run snapshot alone — the path-keyed ownership rows are
written by the deploy and by database-connected workers. Dynamic descriptors and
Windmill-resolved profiles therefore both run there. What an agent does not get is LIVE progress —
that is a per-model event stream, and a round trip per node is the wrong trade —
so its per-model state is settled from `run_results.json` when the run ends, and
its retry state lives only in the worker-local generation. See
[agent-worker-e2e.md](./agent-worker-e2e.md).

The refresh happens **before** the build, from a `dbt parse` with this run's own
vars and env, so a run in flight is already showing the models it is building.

A dynamic descriptor's graph is a property of the RUN, not of the deployed
version, so it is stored per job — see "The graph belongs to a script version"
below. Two concurrent runs of one such script therefore keep their own, and each
run page shows the models that run built.

Re-ingesting is nearly free: the run parses the project (about a second) before
building it and ingests that manifest.

The parse is what makes a newly added model appear in the same run that builds
it, rather than one run late: the graph is written before the build, so the run
page shows the model while it is being built.

## What a share-link viewer sees

A share link is not anonymous access: the token is HMAC'd with the workspace key
and scoped to one job and its descendants. It is an extra grant for a **logged-in
user who lacks access to that job** — which is normally why someone was sent a
link.

Both halves of a dbt run page go through one gate: `/jobs/run_progress/{id}` and
`/jobs/dbt_graph/{id}`, each behind `require_job_read_access`, which validates
the token. The graph then has a second, independent filter — RLS on the `script`
row — and a viewer sent a link usually has no grant there. Deciding the graph's
SHAPE under that filter is wrong: it would answer for the caller's access to the
project rather than for the run they were given, and the Models panel would come
back blank beneath working progress rows.

So a pinned run resolves its version from the JOB ROW, not from `script`: the
`live` CTE takes the path and hash the handler read after authorizing the job.
Two things make that safe rather than a widening:

- **It leaks nothing new.** `v2_job_completed.result` already carries every
  node's `unique_id` and `relation_name`, and this viewer can read it — the
  model set and its relations are already visible to them.
- **`raw_code` is gated separately**, on an `EXISTS` against `script` in the
  authed transaction. The body of a model is the project's source code and stays
  behind access to the project, whatever the shape query resolved.

The path and hash coming from the job row rather than the query also means a
caller cannot pin one project's version while naming another's run.

## What a dbt job returns, and which half of it is a contract

The result is `{engine, engine_version, command, totals, nodes, invocation_args}`,
and each node carries both `status` and `outcome`.

`invocation_args` is the arguments the run used, as SUBMITTED — a `$var:` stays a
reference, so no resolved value is published — and it is omitted when empty. It
exists because a `dbt retry` restores the failed run's arguments inside the
worker and never writes them back to the retry job, whose own args are just
`{"command": {"label": "retry", "dbt_retry_job": "<id>"}}`: the row preview, which
is a `dbt show` of the same project, has nowhere else to get them. On a retry it is therefore ANOTHER
invocation's arguments, which is why a hidden run saves no state at all (see the
retry section).

`status` is dbt's own word, verbatim — `success`, `error`, `partial success`,
`no-op`. It is what the log says and what dbt's docs describe, so it belongs in
the result, but it is dbt's vocabulary and dbt may change it: 1.x and 2.x
already differ on casing, and `no-op` arrived in a minor release.

`outcome` is the same result in Windmill's terms — `passed`, `failed`, `warned`,
`skipped`, `no_op`, `unknown` — and it is the half a downstream script should
branch on. A dbt release that renames a status moves `status` and leaves
`outcome` where it is. Publishing only dbt's word would have made every such
release either a break for users or a lie in our mapping.

## The graph belongs to a script version

`dbt_node` / `dbt_edge` are keyed `(workspace_id, script_path, script_hash,
job_id, unique_id)`. Each deployed version keeps its own graph, and a job records
the version it ran (`v2_job.runnable_id`), so a run page asks for that one:
`/assets/graph?dbt_script_hash=<hex>` renders the project as it was — its models,
its SQL, its `ref()` lineage — instead of whatever is deployed today.

`job_id` is the second half, and it exists for dynamic descriptors only. A
`{{ }}` placeholder in `vars` can enable a different set of models per run, so
those runs re-ingest; keyed by version alone, each re-ingest overwrote the last
and reopening an older run showed the newer run's project, with any model only
the older run built simply gone. A run of a dynamic descriptor therefore writes
its own snapshot under its job id, and its page reads the graph through
`GET /w/{w_id}/jobs/dbt_graph/{id}`, passing the version hash.

A static descriptor writes nothing per run: its graph is the version's, under
the zero-UUID `DEPLOYED_GRAPH` sentinel, and every run of it reads that. The
sentinel is a value rather than NULL because `job_id` is in the primary key and
Postgres does not treat two NULLs as one key, so a re-ingest would accumulate row
sets instead of replacing one. The route falls back to it whenever the job has no
snapshot, which is why a run page can use it unconditionally rather than having
to know whether its descriptor was dynamic.

Pinning to a run is job-scoped, so it is a job route and not a parameter on
`/assets/graph`: it needs the whole job-read contract, which is
`require_job_read_access`. That helper lives in `windmill-api`, which depends on
`windmill-api-assets`, so the read moved to the check rather than the check to
the read. The route charges `assets:read` on top of the `jobs:read` its URL
implies, since the body it returns is asset data.

A snapshot is only written when it DIFFERS from the version's graph, compared by
a digest of the nodes, edges and relation root. Marking a descriptor dynamic is
conservative — a `{{ }}` in `vars` says the arguments reach dbt, not that they
change which models exist — so the usual dynamic run (a date var) resolves to
exactly the graph the deploy stored, and storing that per run would duplicate an
unchanging picture. Those runs write nothing and read the version's graph
through the fallback; only a run whose model set really differs pays.

### Which run's graph becomes what the script owns

Re-ingesting has several causes and they do not want the same thing, so the
reason is carried rather than a bool (`GraphRefresh`):

| Cause | Graph written | Path-keyed `asset` ownership |
|---|---|---|
| Descriptor is dynamic (`{{ }}` in `vars`, `$var:` in `env`) | under the job id | untouched |
| The run overrode `vars` | under the job id | untouched |
| The run narrowed `select`/`exclude` | nothing, unless another cause already made it ingest — then under the job id | untouched |
| The profile moved since the last publish | the **version's** graph | republished |

Ownership follows the version's graph exactly, which is what the first three
rows have in common: the workspace graph takes an asset's relations from the
`asset` rows and its models, SQL, tests and `ref()` lineage from that version's
`dbt_node`/`dbt_edge`, so publishing relations the version's graph does not name
leaves those assets with no model behind them — a placeholder that moves an
alias would empty the current graph of everything dbt contributes to it. A run
storing a snapshot of its own therefore publishes nothing, and an override's
schemas and aliases do not stand as the script's until the next deploy, which is
what a snapshot is for.

The consequence for a dynamic descriptor is that its ownership stays the
deploy's, and a profile that moves under one is settled by a redeploy rather than
by a run: every run of it already shows its own models and re-parses regardless,
so the drift it keeps re-detecting costs it nothing it was not already paying.

The last row is the one that has to publish. The drift check compares the
resolved root against `relation_root_at_last_ingest`, so a run that saw a move and
did not republish leaves the next run seeing the same move — forever, with the
asset rows still naming the old schema and every run paying a `dbt parse` for a
snapshot nobody reads. It rewrites the VERSION's graph rather than a per-run
snapshot for the same reason: once the root is republished no later run detects
the move, so a snapshot would leave those runs reading the pre-move rows.

A snapshot wins where they meet: a drifted run that also overrode its arguments,
or whose descriptor is dynamic, snapshots under its job id and publishes
nothing, and the drift is settled by an ordinary run of a static descriptor or
by a redeploy — a wasted parse per overriding run, where the alternative is one
caller's subset standing as the script's own, or replacing the version's graph
with a picture missing every model that run did not select.

Both halves have a retention story, and they differ because their readers do. A
run's snapshot expires on a clock — 30 days — because the run page that reads it
is transient. A VERSION's graph cannot: its reader is every finished run of that
version, and a run page is as old as its job. So version graphs are bounded by
deploy COUNT instead — the newest 50 per path keep theirs — which makes growth
`versions x models` rather than unbounded in time. Without it a CI deploying on
every commit adds a full model set per commit and nothing ever reclaims it. The
bound is generous on purpose: reaching it empties that version's run pages, so
it exists to stop unbounded growth rather than to be hit in normal use.

Both are pruned by every dbt run, so no background sweep has to know about the
tables. The prune is
deliberately not hung off the progress reporter, which exists only for engines
that emit node events: retention that stops working because an instance chose
Fusion is not retention. A version's own graph lives as long as the version.

### The third provenance: a parse of the editor's buffer

The dbt editor draws a graph of the project **as it is in the editor**, refreshed
on demand by a `dbt_command: "parse"` job over the buffer — the deploy's own
deps → parse → ingest path, with no build. That graph is neither of the two
above: the buffer differs from what is deployed, which is the point of
refreshing it, and a project being written may have no deployed version at all.

So it is keyed to its own PREVIEW JOB with **no version** — `script_hash IS
NULL` — and readable only back through that job id
(`GET /jobs/dbt_graph/{id}`), never through the path. That is what keeps the
property `GraphPublisher::Unversioned` exists for: a parse publishes no
path-keyed `asset` usages and no relation root, so a principal who needs only
`jobs:run` still cannot restate what a deployed project's graph says.

Three consequences of the version being absent:

* **`script_hash` is nullable**, so the primary keys of `dbt_node`, `dbt_edge`
  and `dbt_graph_snapshot` became two partial unique indexes each — versioned
  rows keyed by their version, editor rows by their job alone. The composite
  foreign key to `script` is unchanged: `MATCH SIMPLE` is satisfied by a NULL,
  so a versioned row still cascades with its version and a version-less one is
  outside its reach. A partial arbiter also has to be named, so the marker's
  `ON CONFLICT` repeats `WHERE script_hash IS NOT NULL`.
* **Being outside that cascade, they need clearing by hand.** A route that
  deletes the `script` rows outright reclaims the versioned graph through
  `ON DELETE CASCADE` and deliberately locks nothing ahead of the script row;
  a version-less row references nothing, so it would survive its own script.
  The delete-by-path and bulk-delete routes therefore call
  `clear_dbt_editor_graphs` — AFTER the delete, beside the retry state, since
  every dbt writer takes the script row first and a sidecar taken ahead of it
  deadlocks one of the pair. Archiving clears neither: it leaves the `script`
  row, and both graphs still answer for finished runs.
* **No digest suppression.** A run's snapshot that matches the version's stores
  nothing and reads the version's back; an editor parse always stores, because
  the editor pins to its own job and a suppressed write leaves it nothing to pin
  to — and its provenance label would then claim a parse that is not on screen.
* **Bounded per (path, PRINCIPAL), not by age**: the newest
  `DBT_EDITOR_GRAPHS_KEPT` parses of one script by one identity keep their
  graph, dropped as each refresh lands, since the ones before it are dead the
  moment a newer parse arrives. The principal is load-bearing rather than
  incidental — a preview's PATH is chosen by a caller who needs only `jobs:run`,
  so a count bounded per path alone is a way to retire the graphs of whoever is
  actually editing that script. `permissioned_as` is the execution principal the
  queue derived, which is why `dbt_run_state` keys on it too. The instance-wide
  age sweep every dbt run performs still catches one refreshed once and left.

A `parse` of a job that DOES name a deployed version — the scriptable form, from
a flow or the CLI — writes an ordinary per-run snapshot of that version instead,
suppressed when it agrees with the deploy. Either way it publishes no ownership:
a parse answers for the arguments it was given, so it can no more stand as what
the script owns than an overriding run can.

Which graph is on screen is stated rather than left to be inferred — "parsed
from the editor at 14:32" against "as of last deploy" — from
`dbt_graph_ingested_at` on the graph response. The two are drawn identically, so
without the label the ambiguity the explicit refresh removes would just move
into the editor.

A parse renders `profiles.yml` before dbt runs, so it needs a resolvable
warehouse and a misconfigured project fails a refresh the way it would fail a
run. That is useful early feedback, and the empty state says so.

Per DEPLOY, not per run: ten thousand runs of one version share one graph. The
rows carry a composite foreign key to `script (workspace_id, hash)` with
`ON DELETE CASCADE`, so a version's graph dies with the version and nothing has
to sweep it.

The routes that hard-delete a script rely on exactly that for the VERSIONED
rows and clear none of them. Clearing them first would lock the sidecars ahead
of the `script` rows, the reverse of the order a publication takes — `script`
row `FOR UPDATE`, then the sidecars — and Postgres would abort one of the two
for deadlock. So anything the cascade cannot reach is cleared explicitly and
AFTER the delete, which keeps that order: `dbt_run_state` (keyed by path, no
script key), and the version-less editor graphs, whose NULL `script_hash`
satisfies the composite key without referencing anything (see "The third
provenance" above). `dbt_run_progress` (keyed by job, no key to either) is
reclaimed only by its age sweep.

Two consequences worth knowing:

* **Concurrent deploys no longer race for the graph.** Two versions write
  disjoint rows, so neither can lose. `claim_graph_publication` survives only for
  what is still keyed by PATH — the `asset` usage rows, of which there is one set
  per script — and an older deploy finishing late now records its own graph
  before declining to touch those.
* **A pinned request is scoped differently.** Unpinned, the endpoint scopes by
  the relations in view, using `asset`. Pinned, `asset` is the wrong scope: it
  describes the current deploy, so a model that version had and a later one
  dropped would be filtered out of its own run's graph. The pinned version's
  nodes are the scope instead.

## No cascade from dbt, and no pipeline membership

A finished dbt run does not trigger anything. Its models are recorded, drawn and
tracked; they do not fan out.

A dbt script is also not a pipeline member (`in_pipeline` is forced false for
`ScriptLang::Dbt` at deploy). It materializes warehouse tables, so it looks like
one, but that membership carries an editor whose premise is that you author the
transforms in it — and a dbt project is authored in a local `dbt run` / `dbt
test` loop, with Windmill as the runner and the viewer. Enrolling it put a dbt
project inside the pipeline editor and blurred which of the two a folder holds.
Its models are `dbt://` assets in the shared graph regardless: that is what
puts a native script reading one of them on the same node, and it is independent
of pipeline membership.

dbt already orders its own DAG, so a cascade would only ever add one thing:
waking a Windmill script that reads a mart. That edge is real but narrow, and
only half of it exists — nothing outside dbt can declare a `dbt://` write
(`// materialize` accepts DuckLake targets only), so the reverse direction, an
ingestion script waking a dbt project, cannot be expressed at all.

Against that, dispatching correctly from dbt is not cheap. A run's `select` can
build any subset of the project, so the deploy-time write set is not what ran;
using it wakes consumers of relations the run never touched, and narrowing it
needs a per-job record of what was built, which the per-relation state table
cannot supply (it keeps one row per relation, stamped with the last writer).

So dbt materializes and reports, and `asset_dispatch` returns early for
`ScriptLang::Dbt`. A `# on dbt://<mart>` subscription is refused outright at
deploy rather than accepted and left dormant — an edge drawn on the canvas that
can never fire is worse than an error saying so.

A plain READ still renders the consumer beside the model, which is what makes
the lineage one graph — but it is written in the script's own code, not in a
comment: the body parsers resolve an asset URI from a string literal
(`parse_asset_syntax`), so `"dbt://<resource>/<schema>/<name>"` appearing in a
Python, TS/Bun/Deno, DuckDB or Ansible script is the read. Those four are the
languages with a body-asset parser; the native warehouse ones (snowflake,
bigquery, postgresql, mysql, mssql) declare no assets at all today, so a mart
they consume joins the graph only once that inference exists. Wiring the trigger up later means deciding what a
selective run should notify — that decision is the work, not the plumbing.

## Live per-model progress, and why only dbt-core 1.x has it

`DbtEngine::emits_node_events()` is true for `dbt-core-1x` alone, so only 1.x
moves nodes on the run-page graph while it builds. The other two engines settle
every relation at the end instead.

That is a statement about **where** the engines put their events, not about
whether they produce them. Both Rust engines emit exactly the structured node
events the tailer parses:

```
$ dbt-sa-cli build --log-format json   # and likewise the fusion binary
{"info":{"name":"NodeStart"},"data":{"node_info":{
  "node_status":"started","unique_id":"model.probe.m3",
  "node_relation":{"relation_name":"windmill_dbt_runtime.probe_sch.m3", ...}}}}
```

Measured on 2.0.0-alpha.5 and fusion 2.0.0-preview.202, a three-model project:
15 node events each on the console, 0 in the file log. `--log-format-file json`
is accepted by both — `json` is a listed value — and ignored: the file is text
either way.

The events are therefore only on stdout, which is the human-readable job log.
Taking them would mean setting `--log-format json` and rendering the log
ourselves from each event's `info.msg`, so the run's log stays readable. That
buys live progress on two pre-release engines at the price of permanently owning
log presentation, to work around something upstream has already declared it
intends to support. Not worth it: when either engine honours
`--log-format-file json`, flipping `emits_node_events()` is the whole change,
and the existing tailer starts working untouched.

A finished run is unaffected on every engine — it is coloured from the run's own
result, not from these events (decision 11's note on `run_progress`).

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
└── analytics__dbt/                 the module bundle: the project, unmodified
    ├── wm_dbt.yaml                 the descriptor (the script's content) — OPTIONAL
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

The descriptor is optional, and nothing above it is authored: an unmodified dbt
project is already a complete Windmill script, running the whole project against
the workspace's default warehouse. `wm_dbt.yaml` appears only when the project
needs something Windmill-specific — run arguments, a named warehouse, an engine
pin — and it lives inside the project so that a dbt developer's working copy and
a Windmill bundle are the same directory.

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
is readable from its node in the asset graph. A dbt project has its own editor —
the file tree, the descriptor, the run arguments and the model graph, which is
the artifact's actual shape — but it is not where a dbt project is developed:
that is a CLI loop against a local warehouse (`dbt run --select`, `dbt test`).
Windmill is the runner, the viewer and the place a project is corrected.

**Two scripts against one project means two copies.** Splitting a project across
scripts, so an upstream selection and a downstream one compose, assumed a shared
repository. With bundles they would duplicate the project and drift. Prefer one
script per project with per-run `select`, and treat two scripts as two projects
(decision 6).

**Seeds are the only thing that can bloat a version.** Measured on real dbt code,
`.sql` files run about 500 bytes median and 1.9 KB at p90, so even a 5000-model
project is a few MB before compression. A single committed CSV can exceed all of
it, so the CLI drops any file over 5 MB from the bundle and says which, rather
than counting models.

**Only text is carried.** A dbt project's authored files are text; a binary one
(an image under `docs/`, a stray `.DS_Store`, a parquet seed) is skipped with
the reason. Left in, it would be read as mojibake and, if it carried a NUL,
rejected by Postgres with an opaque `unsupported Unicode escape sequence`.
Binary is detected the way `git` does it, by a NUL in the first 8000 bytes,
because `docs/` and dotfiles do not follow extensions. The push, the staleness
hash and the sync diff share one predicate: a file one drops and another keeps
is a change no push can resolve.

**Secrets are not carried.** `.env`, `.env.*` and `.envrc` are skipped with the
reason. The import above copies whatever the checkout holds, and what a
`.gitignore` was keeping out of the repo is exactly the file that must not
become a script version, readable by anyone who can read the script and handed
back on every pull. dbt does not read them either — `env_var()` takes the
process environment, which Windmill fills from the descriptor's `env` and the
script's environment variables.

**`dbt_project.yml` is rendered before it is read.** dbt allows `env_var()` in
that file, so a project may name its profile or its packages directory through
one. Windmill renders those two settings against the environment the run gives
dbt before acting on them: reading the template instead leaves a rendered
`profiles.yml` keyed under a name dbt never looks up, and a package cache
watching a directory `dbt deps` never fills.

## The script artifact

New `ScriptLang::Dbt`. Content is a YAML descriptor whose field names track dbt's
and Cosmos's vocabulary so the mental model ports without translation:

```yaml
engine: dbt-core-1x               # or dbt-core-2x | fusion
profile:
  warehouse: main                 # a warehouse configured on the workspace, by
                                  # name; omitted takes `main`
  target: prod
  # schema: marts                 # target schema; REQUIRED for BigQuery, whose
                                  # resource is a service-account JSON with no
                                  # dataset in it
  # profiles_yml: profiles.yml    # alternative: keep your own file; it then
                                  # names a warehouse only to say where its
                                  # assets belong (see below)
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
of divergence. One thing is decided before dbt sees them: a run that spells out
`select` or `exclude` drops the descriptor's `selector`, because dbt resolves
`--selector` *instead of* `--select` and passing both would silently build the
descriptor's nodes rather than the ones the run asked for. "Spells out" means
DIFFERS from the descriptor's own value, not merely "was submitted": the
generated run form posts a default back for every field left untouched, and a
selector descriptor's `select` default is `[]`, so reading a submitted `[]` as
an override dropped `--selector` from every run started from the UI, a schedule
or a webhook and built the whole project. A run that wants the whole project
despite the selector asks for it with a selection that differs — `["*"]`.
`select` and `vars` are overridable per run via job args. The **graph** stays the
deployed descriptor's: asset rows are written at deploy, like every other
language's, so a run-arg override changes what gets built without changing what
the graph says the script owns. Split the project into several scripts
(decision 6) when the graph itself should differ.

A `vars` override does re-ingest — vars steer `enabled`, aliases, schemas and
materializations, so the deployed graph would name another run's relations — but
under the job id alone, never as what the script owns: publishing an override's
relations would leave them recorded for the next default run, which then builds
the descriptor's while the graph shows the override's. See "Which run's graph
becomes what the script owns" for the whole table, including the profile move
that is the one cause a run publishes.

`vars` interpolates from job args with `interpolate_template` (`common.rs`,
shared with the Ansible executor). The syntax is `{{ arg_name }}`.

`select`/`exclude` also scope **what the script owns in the graph**, resolved by
asking dbt (`dbt ls --output json`) rather than by interpreting the selector
string. Without that a narrowly-selected script registers as the producer of
every model in the project, and two scripts splitting one project would each
claim all of it. Running several scripts with different selections only composes
because of this.

## Deploy path

New `ScriptLang::Dbt` arm in `worker_lockfiles.rs` (near the `ScriptLang::Ansible`
arm at :2758), producing:

```rust
struct DbtDependencyLocks {
    manifest_digest: String,
    engine: String,
    engine_version: String,
    adapter_version: Option<String>,
    package_lock_digest: Option<String>,
    profile_relation_root: Option<String>,
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

### Dependencies resolve at deploy, and are pinned for every run

A project declaring `packages.yml` ranges or a mutable git revision asks dbt to
*resolve* them, and dbt re-resolves on every `dbt deps`. Windmill resolves once, at
deploy, and pins the result — the same contract every other language's lockfile
gets here.

The deploy records the digest of the `package-lock.yml` dbt produced into
`DbtDependencyLocks`. That digest keys the worker-local package cache and joins the
run identity that gates `dbt retry`. A run restores the tree under that key; a worker
that resolves anything else is refused rather than run, because accepting it would let
one resolution's `run_results.json` decide what a retry rebuilds.

Only a run has a resolution to be held to. The deploy establishes one and accepts
whatever dbt returns — including a `package-lock.yml` dbt rewrites itself, which it
does whenever the `sha1_hash` it stored for `packages.yml` no longer matches. Holding
the deploy to a committed lock would refuse the first deploy after a package is added,
with no way out, since redeploying resolves the same way again.

Consequences worth knowing before choosing whether to commit a lockfile:

- **To pick up a newer version of a ranged dependency, deploy a CHANGE.** Only a
  deploy re-resolves, and an unchanged push is skipped as a no-op — the lock and the
  schema are both derived, so they are compared as they would be stored rather than
  as they arrive. Editing `packages.yml`, or committing the `package-lock.yml` you
  want, is what moves a pinned resolution.
- **A committed `package-lock.yml` lets a deploy hit the cache**, since it is the
  digest the lookup is keyed on before `dbt deps` has run. A project without one, or
  whose committed lock is not what dbt resolves, pays a real `dbt deps` per deploy.
  This is dbt's own recommendation for the same reason.
- **The refusal is per worker, not per script.** `dbt deps` writes its lock into the
  job directory, so a project with a range and no committed lock reproduces its
  resolution only from a cache hit. Once upstream publishes a new version, that
  script keeps running on every worker already holding its tree and fails on the
  first cold one — same commit, same arguments, different outcome by worker. A
  deploy that changes something re-pins and clears it; committing the lock avoids it
  entirely.

Nothing evicts these worker-local caches — package trees, engine installs and retry
state alike — and `cache_clear` does not reach them either: it removes
`$WINDMILL_DIR/cache/`, while all three live under `$WINDMILL_DIR/cache_nomount/`,
as bun's cache does. An operator reclaims them by deleting that directory. Engine
installs dominate the space by two orders of magnitude (~270–290 MB each, bounded
by engine version) and package trees grow one tree per edit of a project that
declares packages. Sweeping either on an age or a size bound is follow-up work.

## Run path

One `dbt build` per job, the shape Cosmos arrived at with `ExecutionMode.WATCHER`
after per-model Airflow tasks proved roughly 6x slower (about 5.5 minutes for one
`dbt run` versus about 32 minutes for 184 per-model invocations on
google/fhir-dbt-analytics). dbt's own threading provides parallelism; Windmill
provides observability.

### The run's arguments are one command block

A run takes a single `command` argument, plus one argument per `{{ placeholder }}`
the descriptor interpolates. `command` is a `oneOf` whose variant IS the command,
so it carries exactly the overrides that command takes:

```jsonc
{"command": {"label": "build", "select": [], "exclude": [], "vars": {}, "full_refresh": false}}
{"command": {"label": "retry", "dbt_retry_job": "019fb410-8ea9-…"}}
{"command": {"label": "show",  "model": "stg_orders", "vars": {}, "limit": 100}}
{"command": {"label": "parse", "vars": {}}}
```

`show` and `parse` are accepted by the worker but are not run-form variants:
each is a thing to do to the project in front of you rather than a job to fill a
form in for, and the graph, the assets list and the dbt editor are where they
live. Both stay reachable from a flow, the CLI and the API, which is what makes
the editor's refresh scriptable and testable rather than a UI-only affordance.

The union is the point: `dbt_retry_job` is required where it means something and
absent everywhere else, `show` takes the ONE model it previews rather than the
`select`/`exclude` pair that narrows a build, `full_refresh` cannot reach a
command that ignores it, and
the run form renders a toggle over the variants rather than a list of fields that
quietly do nothing. The worker spreads the block over the run's arguments to read
them — `label` becomes `dbt_command` — which is why a `{{ placeholder }}` may not
take one of those names (`RESERVED_ARG_NAMES`). What a run SUBMITTED keeps the
block, since that is what `dbt_run_state` saves and `invocation_args` publishes.

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
   works from any worker with a database connection. An agent worker reaches
   the database only through the API, which does not expose this, so it keeps
   only its own local copy; the automatic node retry is refused there for the
   same reason, since its wait could not observe a cancellation. Only `run_results.json` is stored there.
   `dbt retry` also needs `manifest.json`, roughly sixty times larger and
   growing with the project (732 KB against 12 KB on a six-node fixture), but
   the manifest is a pure function of the project files, vars and env — all of
   which the stored identity already pins — so a worker restoring from the
   database re-derives it with a `dbt parse` of about a second. It is a run
   argument (the `retry` command variant) rather than the automatic behavior of
   Windmill's generic retry, which has no per-language hook to change the invoked
   command.
   Each attempt gets a fresh job dir, so the previous run's `target/` is cached
   per (workspace, script) on the worker and restored for a retry.

   **Naming the run.** The `retry` variant requires `dbt_retry_job`, the id of
   the run to resume, and the worker refuses one that is not the run it holds —
   naming both ids, since "that is not the one" is otherwise indistinguishable
   from "nothing is saved". Only the latest failure of a script is kept, so an
   unnamed retry would mean "whatever failed last" and would quietly resume a
   different run than the caller was looking at. The run page's `Resume this run`
   and `Run again → dbt retry with same args` both fill it in; on the run form,
   choosing `retry` prefills it with the run that caller's own retry would land
   on. It is not a selector: naming a run other than the saved one is refused,
   not resumed.

   **Concurrency is the script's, not the retry's.** A retry that starts while
   another run of the same script is in flight rebuilds nodes that run may also
   be rebuilding — appending an incremental model twice. That is what two
   concurrent `build`s of one project do as well: dbt takes no cross-process
   lock, so a project that must not run twice at once sets the script's
   concurrency limit, which covers its retries with it. A lock held across a
   dbt execution instead would outlive worker deaths and cancellations.

   **Who may resume it.** The state is keyed `(workspace, script_path,
   permissioned_as)` — one saved run per script per identity it executes as — so
   anyone entitled to run the script as that principal may resume its last
   failure, which is the same capability as re-running that job: running the
   script requires read access on it, and that access shows them the run and its
   arguments already. Naming the run neither widens nor narrows that; the id is
   checked against what the state holds, not against the caller.

   For an `on_behalf_of` script that means every caller shares one saved run,
   since they all execute as the owner — deliberately, since the state describes
   the script's last run under the owner's identity rather than any one caller's.

   **What a retry actually adds, and where that crosses a line.** Resuming grants
   no capability a caller lacks: they may already run the script as that principal,
   and a plain run builds everything the descriptor selects, of which a retry
   rebuilds a subset. The one thing it adds is information — the result carries
   `invocation_args`, the resumed run's arguments as SUBMITTED, because a retry
   job's own args are just the command and the run it names, and the row preview
   needs the real ones.

   For almost every shape the caller could already read that run, so nothing
   crosses: under `f/`, `see_folder_extra_perms_user` makes a job readable to
   everyone with read on the folder, which is also what grants execution; an
   ordinary script's runs are `permissioned_as` the caller, and another user's runs
   are keyed under their own principal and unreachable. It takes all three of a
   `u/<owner>` path, `extra_perms` sharing and `on_behalf_of` for "may run as this
   principal" to be broader than "may read this principal's jobs" — and there a
   grantee learns the literal argument values of another's run. References stay
   references, so no resolved secret is among them.

   That residual is accepted rather than gated. A gate needs the caller's identity,
   and a worker has only `created_by` — `display_username()`, which a token LABEL
   supplies. Resolving it as a username denies every labelled token (a CI token
   becomes `label-<name>`, which is no workspace member) and still trusts a name;
   authorizing where the caller is real means the submitting path, not the worker.
   The exposure did not justify either.

   That equivalence holds only while the run is READABLE, so the one run that
   breaks it saves nothing: a job pushed `invisible_to_owner` is hidden from the
   script's owners, and a retry publishes the arguments it restored, which would
   make that retry the one way to see them. A hidden run therefore keeps no
   retry state at all — it cannot be resumed by anyone, including whoever
   launched it, which is the cheaper half of the trade. Keying by the initiating
   caller would not have worked instead: `created_by` is `display_username()`,
   which a token LABEL supplies, so two callers can share one value and one can
   name a third person (GHSA-8x8x-88qc-qp4r, whose fix was to stop trusting that
   name for authorization). A retry does name the run it resumes, but that name is
   checked against the saved state rather than authorized as a job read — doing
   the latter, which is what would let a hidden run be resumed by its own author,
   needs the submitting path, where the caller is real.
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
| model relation | `dbt://` asset | new `AssetKind` |
| `ref()` graph | lineage edges | `replace_static_asset_usage` |
| `materialized: table` | `materialize_strategy: replace` | `AssetGraphRunnableNode` |
| `materialized: incremental` | `append` or `merge` (by `unique_key`) | same |
| `{% snapshot %}` | `scd2` | same, incl. `<dim>_current` handling |
| `unique`/`not_null`/`accepted_values`/`relationships` | `data_tests` | exact 1:1 with the four `// data_test` kinds |
| declared column metadata | `columns` on the asset node | descriptions only; see the note below |
| model `tags` | node badge | `tag` |
| source freshness | `freshness` | `last_success_at` chip |
| `run_results.json` | materialization records | `record_materialization` |
| `dbt_packages/` | worker-local cache | keyed by `packages.yml`, the project digest and the `package-lock.yml` the deploy resolved |

## Phases

**Phase 1: run it.** `ScriptLang::Dbt` across the 41 `ADD_NEW_LANG` sites (mostly
one-liners in `EditorBar.svelte`, `scripts.ts`, `script_helpers.ts`,
`LanguageIcon.svelte`, `script_common.ts`). Engine provisioning for all three
options in `Dockerfile` and `docker/DockerfileFull*` (bundle 1x and 2x, fetch
Fusion at runtime). New `backend/windmill-worker/src/dbt_executor.rs`: descriptor
parse, bundle materialisation, `profiles.yml` render, `dbt build`, log
passthrough, structured result, retry.

**Phase 2: graph.** `DbtDependencyLocks` and the deploy arm. Migration via
`cargo sqlx migrate add -r dbt_runtime`. New `dbt://` `AssetKind` with
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
6. **Shared node**: a native script that READS a mart renders as a reader of the
   same node the dbt model writes — one node, not two islands. Declared with a
   plain read (`# dbt://<mart>`), never `# on`: a `dbt://` subscription is
   refused at deploy, because nothing but dbt writes a warehouse relation and a
   dbt run does not dispatch (see "no cascade from dbt").
7. **Selection**: descriptor `select`/`exclude`, and a run-arg override, each
   build only the expected subset.
8. **Dynamic descriptors**: a `{{ }}` placeholder in `vars` re-ingests the graph
   from the run's own manifest, so a model that placeholder enables appears in
   the same run that builds it.
9. **Both credential paths**: resource-rendered `profiles.yml`, and the project's
   own `profiles.yml` with env-var injection.
10. **Caching**: a second run reuses the cached `dbt_packages/` with no network
    fetch.

Keep only tests that pin behavior a future change could break. Per AGENTS.md,
delete development scaffolding before marking the PR ready.
