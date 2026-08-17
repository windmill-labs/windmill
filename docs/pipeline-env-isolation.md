# Fork data environments for pipeline materialization ("dev data")

Status: **implemented** (branch `pipeline-env-isolation`). Companion to
[`ducklake-materialization.md`](./ducklake-materialization.md).

## The problem

Workspace forks fork the CODE, but `// materialize ducklake://<lake>/<table>` targets are
absolute pointers into shared physical storage. A fork clones `workspace_settings.ducklake`
verbatim, so `transform_attach_ducklake` resolved a fork's `ducklake://main` to the **parent's
catalog database and S3 data path** — deploying or running a materializing pipeline inside a
fork wrote the production tables. dbt answers this with target schemas + `--defer`; datatables
already have a fork answer (`forked_datatables` → `wm_fork_*` databases + a drop endpoint);
ducklake had none. `wmill pipeline run --local` only covers preview runs, and previews still
resolve targets against the real workspace.

## The design

Everything hangs off two DuckLake ATTACH capabilities (verified in a spike, DuckDB 1.5.4 +
current ducklake extension): `METADATA_SCHEMA` (the catalog's metadata tables live in a chosen
pg schema of the same catalog DB — auto-created on first attach; default is `public`) and
`READ_ONLY`, plus catalog-stored views that may reference other attached catalogs and rebind
per session.

### Fork-time choice: isolated (default) vs shared, per lake

The fork-creation dialog offers a per-lake data-environment choice (the section renders only
when the parent workspace has lakes configured, mirroring the datatable section):

- **Isolated** (default — also what unlisted lakes and API callers that omit the field get) —
  the full behavior described below.
- **Shared** — explicit opt-out: the fork reads and writes the parent's lake directly through
  its cloned config, like a non-fork workspace (e.g. a fork meant to run prod-equivalent
  backfills). Stamped as `fork_behavior: "shared"` on the lake entry in the fork's own
  `workspace_settings.ducklake` at creation (`shared_ducklakes` on the create-fork request);
  the resolver, the graph indicator, and (trivially — nothing registers) cleanup all key off
  it. Shared also lifts the mysql-catalog restriction, since nothing needs namespacing.

Absent-field-means-isolated keeps every pre-existing fork and API caller on the safe path.
Sharing is a per-fork-creation choice, never inherited: the settings clone copies the source's
config verbatim, so fork creation strips any cloned `fork_behavior` stamps before applying its
own `shared_ducklakes` list — a fork of a shared fork defaults back to isolated. A
shared fork ancestor in a fork chain resolves like a root (its data lives at its config's
default location). Flipping a fork's lake to `shared` later (settings edit) orphans any
namespace it already materialized until fork deletion cleans it — the registry row survives
the flip.

### Write redirect — one seam, no grammar changes

`get_ducklake_from_db_unchecked` (windmill-common/src/workspaces.rs) is fork-aware: when the
resolved workspace has a `parent_workspace_id` (throwaway `wm-fork-*` fork or standing dev
workspace), the returned `DucklakeWithConnData` is namespaced **in its existing fields**:

- `extra_args` += `METADATA_SCHEMA 'wm_fork_<mangled wid>_<mangled lake>_<hash8>'` —
  injective, ≤63 chars, stable across releases (persisted state depends on it), and
  **lake-scoped**: two lakes of one workspace may share a catalog database, and a
  per-workspace schema would merge their namespaces in the fork;
- `storage.path` = `__wm_forks/<fork segment>/<original path>` — the segment is the
  mangled+hashed workspace id collapsed to ONE path component (fork ids are only
  git-branch-safe and may contain `/`; used raw, `wm-fork-a/b` would nest inside sibling
  `wm-fork-a`'s cleanup prefix) — a **bucket-root prefix**, not a
  sub-path of the parent's DATA_PATH, because lake maintenance
  (`ducklake_delete_orphaned_files`, snapshot expiry) scans everything under the parent's path
  and would delete live fork files nested there.

Folding the redirect into existing fields means the EE agent-worker endpoint (which just
serializes this struct) needs **no changes**, and an agent worker running an older binary still
writes the fork namespace — it only misses the defer views (reads fail loudly; never a prod
write). Every ducklake access funnels through the same ATTACH transform — the managed
materialize's synthetic `_wm_target` attach, `materialize manual`, raw user SQL, and all UI
preview / History / backfill surfaces (they are `WM_INTERNAL_DB_*` marker preview jobs) — so a
fork cannot reach the parent namespace through the managed path at all.

Hardening: in fork mode, user- and settings-supplied `METADATA_SCHEMA` / `DATA_PATH` /
`OVERRIDE_DATA_PATH` ATTACH options are stripped before injection (DuckDB silently keeps the
*last* occurrence of a duplicated option, so a duplicate would escape the namespace).
MySQL-catalog lakes error loudly in forks (no pg schemas to namespace with) rather than ever
writing the shared catalog.

### Read-defer — iterate on one node without rebuilding upstream

In a fork, `transform_attach_ducklake` additionally emits:

1. `ATTACH IF NOT EXISTS … AS __wm_dl_<lake>_<ancestor>_<hash> ([<ancestor's own args>, ]
   DATA_PATH <ancestor's>, OVERRIDE_DATA_PATH TRUE, READ_ONLY[, METADATA_SCHEMA
   <ancestor's>])` per fork-chain ancestor, resolved from the **ancestor's own settings**
   (fork-side settings drift can't repoint what "parent" means). The ancestor config's
   non-reserved `extra_args` (e.g. `ENCRYPTED true`) ride along — FIRST, so the fork-owned
   options win under DuckDB's last-occurrence-wins. No
   `AUTOMATIC_MIGRATION`/`CREATE_IF_NOT_EXISTS` — a fork job must never mutate an ancestor
   lake. `READ_ONLY` makes prod writes *physically impossible* through ducklake, including in
   `materialize manual` and raw SQL. Fork ancestors whose namespace was never bootstrapped are
   skipped, with existence checked in each ancestor's OWN catalog database (a fork whose
   catalog drifted away from an ancestor's must not misread that ancestor's namespace as
   missing); an unreachable ancestor catalog only disables that ancestor's defer.
2. `CREATE VIEW IF NOT EXISTS <alias>.<table> AS SELECT * FROM <owning ancestor
   alias>.<table>` for every **deferred table**: (`materialized_partition` rows with
   `status = 'materialized'` anywhere in the ancestor chain) MINUS (the fork's own rows). Each
   view targets the NEAREST ancestor that physically owns the table
   (`ForkDeferTable.ancestor_idx`): in a `fork → parent → root` chain where only the root
   materialized a table, the parent has no copy (it defers too), so a view over the parent
   would not bind. SCD2 owners (detected by the `is_current` column in the owning ancestor's
   latest captured `materialized_asset_schema`) also get the `<table>_current` companion view.
   So a consumer reading `dl.orders` transparently reads the nearest owning ancestor's current
   data until the fork materializes `orders` itself.
3. When the running job's managed materialize targets a table that currently exists **as a
   view** in the fork namespace, the view (and its `_current` companion) is dropped after the
   target attach — the first fork materialize replaces the defer view with a real fork table
   (`CREATE [OR REPLACE] TABLE` refuses to replace a view — verified — so without the drop the
   transition fails; with it, `sql_materialize.rs` codegen needs no fork awareness at all).
   The is-it-a-view decision keys on the catalog's ACTUAL live views (`ducklake_view`, read at
   resolution time into `fork_defer.fork_views`), **not** on recorded materialization status:
   after a failed run the status cannot distinguish a defer view from a real table, and a
   mismatched `DROP VIEW` (or skipped drop) would wedge the asset on every retry — found live
   in e2e when a storage failure flipped a fork asset's status to `Failed`.

The graph shows the state per ducklake asset in a fork: an amber ↗ chip = *deferred* (some ancestor in the chain
owns the table),
an emerald fork chip = *fork-materialized* (`fork_materialization` on the `/assets/graph`
response, computed from fork + parent `materialized_partition`; the parent's rows are read on
the plain pool since fork membership doesn't imply parent membership). The details pane shows
the equivalent banner.

### Lifecycle & cleanup

A sidecar registry `fork_ducklake_namespace(workspace_id, ducklake_name,
metadata_schema, catalog, storage, storage_ref, data_path)` is upserted (behind a TTL'd in-process cache, keyed
per workspace and invalidated whenever a workspace's registry rows are cleaned up — a same-id fork recreated
within the TTL must re-register or its namespace would be orphaned at its own deletion) on fork
resolution, **one row per physical location ever attached** — its PK includes
`(catalog, storage, storage_ref, data_path)`, so if the fork's lake settings drift, later attaches add rows rather
than replace them and cleanup covers every location the fork wrote, not just the first — connecting to the
REGISTERED catalog identity and deleting files from the REGISTERED storage identity
(`storage_ref`, resolved from the logical storage name at attach time), never whatever the
fork's settings point at by deletion time. Explicit
`GRANT`s ship in the same migration; the registry is not cloned into sub-forks.

The registry has deliberately **no FK to `workspace`**: rows are the durable cleanup ledger and
outlive the workspace row when physical cleanup fails after the delete commits (unreachable
catalog, storage outage) — each row is deleted only after its schema + data cleanup succeed.
Fork **creation** retries any leftover rows for the reused id and refuses to proceed while a
metadata schema still cannot be dropped, so a recreated same-id fork can never silently
reattach stale tables. Data-file leftovers alone don't block: a deleted fork's `$res:` storage
resource is gone forever, but once the schema is dropped the files are inert — the surviving
row keeps them tracked and the next successful cleanup of the same prefix (typically the
recreated fork's own deletion, with live credentials) sweeps them.

Retries after the fork is gone get two aids: the row's `schema_dropped` phase flag (set when
the schema dropped but data cleanup failed; retries then skip the schema phase and need no
catalog credentials — registration resets it on re-attach, which recreates the schema), and
`$res:` fallback resolution against the workspace being forked (the deleted fork's resources
were clones of a parent's, so the new parent is the natural credential donor).

`POST /w/{fork}/workspaces/drop_forked_ducklake_namespaces` (same permissions as
`delete_workspace` via a shared gate: fork owner or superadmin, plus parent-prod-admin when the
fork is an attached dev workspace) drops each registered pg metadata schema
(`DROP SCHEMA … CASCADE` on the catalog connection, hard-guarded on the `wm_fork_` prefix) and
deletes the `__wm_forks/<fork segment>/…` objects in the workspace storage (hard-guarded on that
prefix; `parquet` feature). The fork-deletion UI calls it next to
`drop_forked_datatable_databases`, for the fork and for deleted child forks. Registry rows are
deleted per-lake only after both cleanups succeed, so a partial failure is retryable.

## Semantics & caveats (by design)

- **Orphaned forks stay isolated.** `parent_workspace_id` is `ON DELETE SET NULL`, so a
  `wm-fork-*` workspace can outlive its parent. Its ancestor chain is then empty, but its cloned
  config still points at the shared lake — so fork resolution keys on the `wm-fork-` prefix as
  well as the chain (mirroring `workspace_is_fork`). An orphaned fork gets the write redirect,
  registration and cleanup but no defer views; pre-orphaning defer views fail loudly at bind and
  flip to real tables on first materialize. The same rule applies to a fork's *ancestors*: a
  last-in-chain `wm-fork-*` ancestor is an orphaned fork (not a root) and its READ_ONLY attach
  targets its fork namespace, not its cloned base config. Prefix-less dev workspaces cannot be
  orphaned (deleting their prod is blocked while attached).
- **Defer is live, not snapshot-pinned.** A deferred read sees the parent's *current* table —
  the same trade dbt `--defer` makes. Snapshot-pinned defer could later ride on the recorded
  `snapshot_id`s.
- **Defer granularity is per-table.** Materializing one partition in the fork makes the fork's
  table authoritative for *all* partitions (the rest are empty until backfilled in the fork).
- **Incremental strategies start empty in a fork.** `merge`/`append`/SCD2 create a fresh table
  on first fork run — no parent rows are inherited, and fork SCD2 history diverges from the
  parent's. Copy-on-fork seeding would be a follow-up opt-in.
- **Only tracked tables defer.** A parent table with no `materialized_partition` row (raw SQL
  writes outside `// materialize`) or whose latest run `Failed` is not deferred — defer views
  bind at CREATE against the parent catalog, and a view on a missing table would fail every
  unrelated fork job. Such tables read as absent in the fork.
- **Transient race.** Between a fork materialize's commit and its state recording, a concurrent
  fork job may still emit `CREATE VIEW IF NOT EXISTS` for that table — which silently yields to
  an existing table (verified), so the race is a no-op.
- **Accident prevention, not a security boundary.** A fork holds the parent's cloned
  credentials by design; a determined fork user can still `ATTACH 'postgres:…'` with the cloned
  catalog resource or `COPY TO 's3://…'` directly. The isolation guarantees are scoped to the
  managed ducklake path (where they are physical: READ_ONLY parent attach + separate metadata
  schema + separate data prefix).
- **Fork chains** compose: each ancestor namespace is attached under a deterministic alias, and
  a parent's own defer views (referencing *its* parent's alias) rebind correctly in the
  grandchild's session because the whole chain is attached.
- **Datatables are out of scope** — the existing `forked_datatables` chooser at fork creation
  covers them (`keep_original` remains their default, which shares the parent DB; picking
  `schema_only` there is the datatable analog of this feature).

## Key files

- `backend/windmill-common/src/workspaces.rs` — fork resolution (`fork_scoped_ducklake`),
  `fork_ancestor_chain`, naming (`fork_ducklake_metadata_schema`,
  `fork_ducklake_ancestor_alias`, `fork_data_path`), option stripping, registry.
- `backend/windmill-common/src/materialization.rs` — `list_fork_defer_tables`.
- `backend/windmill-worker/src/duckdb_executor.rs` — `fork_defer_statements` (ancestor
  attaches, defer views, target transition), user-arg stripping.
- `backend/windmill-api-assets/src/lib.rs` — `fork_materialization` on the graph.
- `backend/windmill-api-workspaces/src/workspaces_extra.rs` —
  `drop_forked_ducklake_namespaces`.
- `backend/windmill-api-workspaces/src/workspaces.rs` — `clone_asset_usages_and_triggers`:
  `asset` + `script_trigger` rows are deploy-derived and were never cloned into forks, so a
  fork's pipeline graph had no asset nodes/edges and its dispatch cascade never fired (fixed
  here because this feature is unusable without it).
- `backend/migrations/20260703170745_fork_ducklake_namespace.*` — registry + GRANTs.
- Frontend: `AssetGraph/{types.ts,AssetNode.svelte,AssetGraphCanvas.svelte,
  AssetGraphDetailsPane.svelte}`, `pipeline/[folder]/+page.svelte`,
  `sidebar/SidebarContent.svelte` (fork-delete wiring).
