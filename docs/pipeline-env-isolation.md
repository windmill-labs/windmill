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

Absent-field-means-isolated keeps every pre-existing fork and API caller on the safe path. A
shared fork ancestor in a fork chain resolves like a root (its data lives at its config's
default location). Flipping a fork's lake to `shared` later (settings edit) orphans any
namespace it already materialized until fork deletion cleans it — the registry row survives
the flip.

### Write redirect — one seam, no grammar changes

`get_ducklake_from_db_unchecked` (windmill-common/src/workspaces.rs) is fork-aware: when the
resolved workspace has a `parent_workspace_id` (throwaway `wm-fork-*` fork or standing dev
workspace), the returned `DucklakeWithConnData` is namespaced **in its existing fields**:

- `extra_args` += `METADATA_SCHEMA 'wm_fork_<mangled wid>_<hash8>'` — injective, ≤63 chars,
  stable across releases (persisted state depends on it);
- `storage.path` = `__wm_forks/<fork wid>/<original path>` — a **bucket-root prefix**, not a
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

1. `ATTACH IF NOT EXISTS … AS __wm_dl_<lake>_<ancestor>_<hash> (READ_ONLY, DATA_PATH
   <ancestor's>, OVERRIDE_DATA_PATH TRUE[, METADATA_SCHEMA <ancestor's>])` per fork-chain
   ancestor, resolved from the **ancestor's own settings** (fork-side settings drift can't
   repoint what "parent" means). No `AUTOMATIC_MIGRATION`/`CREATE_IF_NOT_EXISTS` — a fork job
   must never mutate an ancestor lake. `READ_ONLY` makes prod writes *physically impossible*
   through ducklake, including in `materialize manual` and raw SQL.
2. `CREATE VIEW IF NOT EXISTS <alias>.<table> AS SELECT * FROM <parent alias>.<table>` for
   every **deferred table**: (parent's `materialized_partition` rows for this lake with
   `status = 'materialized'`) MINUS (the fork's own rows). SCD2 parents (detected by the
   `is_current` column in their latest captured `materialized_asset_schema`) also get the
   `<table>_current` companion view. So a consumer reading `dl.orders` transparently reads the
   parent's current data until the fork materializes `orders` itself.
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

The graph shows the state per ducklake asset in a fork: an amber ↗ chip = *deferred to parent*,
an emerald fork chip = *fork-materialized* (`fork_materialization` on the `/assets/graph`
response, computed from fork + parent `materialized_partition`; the parent's rows are read on
the plain pool since fork membership doesn't imply parent membership). The details pane shows
the equivalent banner.

### Lifecycle & cleanup

A sidecar registry `fork_ducklake_namespace(workspace_id FK ON DELETE CASCADE, ducklake_name,
metadata_schema, storage, data_path)` is upserted (behind an in-process cache) on first fork
resolution per lake — it records exactly what to clean even if settings drift later. Explicit
`GRANT`s ship in the same migration; the registry is not cloned into sub-forks.

`POST /w/{fork}/workspaces/drop_forked_ducklake_namespaces` (same permission as
`delete_workspace`: fork owner or superadmin) drops each registered pg metadata schema
(`DROP SCHEMA … CASCADE` on the catalog connection, hard-guarded on the `wm_fork_` prefix) and
deletes the `__wm_forks/<wid>/…` objects in the workspace storage (hard-guarded on that
prefix; `parquet` feature). The fork-deletion UI calls it next to
`drop_forked_datatable_databases`, for the fork and for deleted child forks. Registry rows are
deleted per-lake only after both cleanups succeed, so a partial failure is retryable; the rows
otherwise die with the workspace via FK cascade.

## Semantics & caveats (by design)

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
