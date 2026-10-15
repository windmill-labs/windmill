# DuckDB isolation and the patched engine

DuckDB runs **in-process** in the worker, through the FFI cdylib in
`backend/windmill-duckdb-ffi-internal`. It is therefore the one language that the
nsjail/unshare isolation cannot confine: there is no child process to jail. On a worker
that isolates jobs, `windmill-worker/src/duckdb_isolation_ee.rs` mirrors that policy at
the connection level instead, rewriting the block list handed to the engine so it starts
with three irreversible settings:

```sql
SET lock_temp_directory=true;
SET disabled_filesystems='LocalFileSystem';
SET allow_community_extensions=false;
```

`disabled_filesystems` is the fence. `lock_temp_directory` is what keeps out-of-core
execution working behind it, and it is **not a stock DuckDB setting** — see below.

## Why the engine had to change

DuckDB spills through the same file system a query would use to read `/etc/hostname`, so
`disabled_filesystems='LocalFileSystem'` also stops the buffer manager. Every query whose
working set outgrows `memory_limit` then fails instead of going out-of-core. No
combination of stock settings gives "spill, but no other local file access":

- `allowed_directories` / `allowed_paths` enforce nothing while `enable_external_access`
  is true — `DBConfig::CanAccessFile` returns early.
- Turning `enable_external_access` off does preserve spilling (startup adds the temp
  directory to `allowed_directories`), but the postgres and mysql extensions refuse to
  `ATTACH` on that flag alone, with no allowlist entry able to re-permit it. That would
  take DuckLake catalogs and datatables down with it.
- A remote `temp_directory` is impossible: DuckDB cannot open an HTTP file for both
  reading and writing, and the attempt aborts the process.

The engine is also not arranged the way the layering suggests. `FileSystem::GetLocal(db)`
looks like an escape hatch — it is what secret storage and extension installation use —
but `LocalDatabaseFileSystem::GetFileSystem()` re-applies the `disabled_filesystems`
check by hand, so it is not one. (That is also why extension autoload broke behind the
fence in DuckDB 1.5.3, which `EXTENSION_ALLOWLIST` and `PRELOAD_EXTENSIONS` exist to work
around.) The buffer manager, meanwhile, reaches its temp files through
`FileSystem::GetFileSystem(db)`, an opener-wrapped virtual file system, so both the path
allowlist and the disable apply to it.

## What the patches add

`lock_temp_directory` (BOOLEAN, default false, global, irreversible). Setting it:

1. fixes `temp_directory` in place — it can no longer be modified, and the setting itself
   cannot be unset or reset;
2. fixes `max_temp_directory_size` the same way, so the ceiling on what may be written
   there cannot be raised either;
3. routes the buffer manager's temporary files through a local file system that
   `disabled_filesystems` does not apply to.

The halves are what make it safe as a set. The exemption is only sound because the
directory can no longer be redirected: without (1), a script could point `temp_directory`
at an arbitrary path and get a blind write primitive through the exemption. And it is only
bounded because of (2): DuckDB accepts `SET max_temp_directory_size='100GB'` from any
statement, so a cap a script can lift is not a cap.
`allowed_directories` still applies either way.

The isolation transform therefore emits `SET lock_temp_directory=true` **before**
`disabled_filesystems`, while the local file system is still reachable, and after the FFI
has already pointed `temp_directory` at the job dir
(`configure_duckdb_resource_limits`). Everything a script authored runs after all three
settings, so a script can neither move the temp directory nor raise the cap on it.

## Bounding the spill

`configure_duckdb_resource_limits` sets `max_temp_directory_size` from
`DUCKDB_MAX_TEMP_DIRECTORY_SIZE`, immediately before it takes the lock. Unset — the
self-hosted default — leaves DuckDB's own default of 90% of the temp volume's free space,
so nothing changes for an install that does not opt in. The cloud sets `6GiB` per worker
pod (`cloudee/cloud/cloud.yml` in `infra-aws`); those pods run a single worker each, so it
is effectively a per-pod cap and is not divided by `NUM_WORKERS`.

Those pods carry no `ephemeral-storage` limit, so what the cap has to stay clear of is the
node's eviction threshold rather than a per-pod one, and the budget is tighter than the
node's size suggests: a worker pod already holds 4.7-6.9 GiB of language-runtime caches and
job dirs before any spill, so a ~100 GB node running 5-6 of them sits at 45-60 GB used at
rest. Every pod on the fullest node spilling to its cap at the same instant would land near
the kubelet's default `nodefs.available<10%`. That is the tail, not the common case, and it
is still bounded where the previous behaviour was not — one query could take all the free
space by itself — but the cap is sized against a mostly-full node, not an empty one. Re-measure
before raising it, and re-measure if the runtime-cache baseline grows.

It is read from the environment rather than passed across the FFI: the cdylib is versioned
separately from the worker and agent workers ship it out of band, so a signature change
would mean an ABI bump and a coordinated redeploy for what is a config value.

The value is DuckDB's spelling, not Kubernetes': `KiB`/`MiB`/`GiB`/`TiB` for 1024-based
units, `KB`/`MB`/`GB`/`TB` for 1000-based. Kubernetes' `6Gi` is not a unit DuckDB knows,
and an unparseable value fails every DuckDB job on the worker rather than being ignored —
a cap that quietly disappeared would be worse. `DUCKDB_MEMORY_LIMIT` is spelled the same
way and DuckDB's parse error says only "memory", so the cap's `SET` is issued on its own
statement: batched together, a bad value in either would produce the same message and name
neither.

Enforcement is graceful. A query that needs more spill than the cap fails with
`Out of Memory Error: failed to offload data block of size … This limit was set by the
'max_temp_directory_size' setting.`, its temp files are removed, and the worker carries on.
Size it from measurement, not from the data: DuckDB compresses temp files, so the logical
spill a given cap permits is well above the number set.

That engine message goes on to suggest `PRAGMA max_temp_directory_size='10GiB'`, which the
lock refuses — advice that was actionable before the cap was frozen and is now a dead end
ending in a second, unrelated-looking permission error. `decode_ffi_error`
(`duckdb_executor.rs`) appends a correction whenever the message appears, on every worker
rather than only isolating ones, since the lock is taken on every connection. It matches
DuckDB's wording verbatim, which
`spilling_past_the_configured_cap_fails_the_query_and_cleans_up` asserts against real engine
output so an engine bump that rewords it fails the test instead of silently dropping the
correction.

The engine emits that same message for **its own default cap**, so which limit was hit
cannot be read off the message: with `DUCKDB_MAX_TEMP_DIRECTORY_SIZE` unset it means the
worker is low on disk, not that anyone configured a ceiling. `spill_cap_hint` takes the
answer from the worker's config instead and says whichever is true — naming the env var on a
worker that never set it would point the reader at a knob that does not exist.

## Where the patched engine comes from

`backend/windmill-duckdb-ffi-internal` depends on
[`windmill-labs/duckdb-rs`](https://github.com/windmill-labs/duckdb-rs), branch
`windmill/duckdb-1.5.5`, pinned by rev. That fork is duckdb-rs 1.10505.0 (DuckDB 1.5.5)
plus the windmill commits; `duckdb.tar.gz` is left byte-identical to upstream's and
`crates/libduckdb-sys/windmill/0*.patch` are applied to the extracted sources at build
time, in order, so `git diff` against `duckdb/duckdb-rs` shows the whole delta as text.
`0001` adds the setting and the exemption, `0002` extends the lock to
`max_temp_directory_size`. `crates/libduckdb-sys/windmill/README.md` in the fork carries
the rebase recipe.

### The patches may not move a struct member

The engine loads the **prebuilt** extensions from `extensions.duckdb.org` — httpfs, azure,
postgres, ducklake, parquet, and the rest of `EXTENSION_ALLOWLIST` — which are compiled
against the struct layouts of the release the engine claims to be. A patch that shifts a
member those extensions reach breaks them, and not loudly: the extension reads the
neighbouring field and `LOAD httpfs` **blocks forever** rather than failing, so every
DuckDB job hangs until its timeout.

The first revision of `0001` declared its new `DatabaseInstance` member next to
`local_db_file_system`, which pushed `create_api_v1` — the extension C-API entry point —
8 bytes along, and every extension load hung. It is declared last for that reason, and
`lock_temp_directory` is declared at the end of `DBConfigOptions`' run of bools so it takes
a byte of padding the struct already had.

The invariant is **no pre-existing member changes offset**, which is strictly stronger than
`sizeof` holding — the first attempt at that field went in next to `temporary_directory`,
kept `sizeof(DBConfigOptions)` at 920 by consuming interior padding, and still slid four
bools along by a byte each. A rebase must diff the full record layouts
(`clang++ -Xclang -fdump-record-layouts`, recipe in the fork's README), not the sizes.
`prebuilt_extensions_still_load` in the FFI crate is the end-to-end guard — a timeout, not
an assertion, because the failure mode is a hang — but it covers extension *entry*, not
every field an extension reads, so it does not substitute for the layout diff.

Alternatives considered and rejected: vendoring a patched `libduckdb-sys` in this repo
would put a 6 MB binary tarball into windmill's history on every engine bump and would
need the FFI builder stage in `Dockerfile` to copy more than
`backend/windmill-duckdb-ffi-internal`; linking a prebuilt engine through
`DUCKDB_LIB_DIR` would drop the `bundled` build, so every developer and every CI job
would need a patched `libduckdb` on the box before `build_dev.sh` could work.

## What stops the patches disappearing silently

An unpatched engine compiles, links and passes every existing test. Three things catch it:

- **The fork's build script** asserts each patch's marker string is present in the patched
  sources after applying it, and panics if one no longer applies.
- **`patched_engine_tests` in the FFI crate** runs a query that must spill, first
  without the lock (asserting the fence stops it, which is what proves the query really
  spills) and then with it (asserting it succeeds); it also pins that the lock leaves
  neither `temp_directory` nor `max_temp_directory_size` settable, and that a configured
  connection stops at its cap. It fails outright on a stock engine, where the `SET` is an
  unrecognized parameter. CI runs it via the FFI crate's own `cargo test` step in
  `backend-test.yml`.
- **The transform itself fails closed**: on a stock engine `SET lock_temp_directory=true`
  errors, so every DuckDB job on an isolating worker fails immediately rather than
  quietly losing the ability to spill.

## Upstream

Both patches are written against `duckdb/duckdb` `v1.5.5` in the shape upstream takes
changes. Besides the engine sources they carry `src/common/settings.json` (the settings
generator's input, regenerated with `scripts/generate_settings.py` and formatted with
clang-format 11.0.1), two sqllogictests under `test/sql/settings/`, and one line in
`test/api/test_reset.cpp` — that test requires every setting to survive a `RESET`, so an
irreversible one has to join the exclusion list `disabled_filesystems` and
`lock_configuration` are already on. They are `git format-patch` files, so they apply with
`git am`:

```bash
git clone https://github.com/duckdb/duckdb.git && cd duckdb && git checkout v1.5.5
git am .../crates/libduckdb-sys/windmill/0*.patch
GEN=ninja make && ./build/release/test/unittest "test/sql/settings/lock_temp_directory*"
```

`0001` is filed as
**[duckdb/duckdb#24694](https://github.com/duckdb/duckdb/pull/24694)**, against `main`
rather than `v1.5.5`. Filing was a human decision on purpose: their `CONTRIBUTING.md` asks
contributors not to submit LLM-generated pull requests, to discuss the change on GitHub
first, and to run CI on a fork before opening one.

**`0002` is windmill-only and is not on #24694.** It is kept as a separate commit rather
than folded into `0001` for exactly that reason: `0001` stays byte-identical to what the
open PR carries, and `0002` applies to it verbatim as a second commit whenever a human
chooses to push it. That is the one sanctioned divergence; when it is pushed, say so here
and in the fork's `windmill/README.md`, and the "same change" rule below covers it too.

**[duckdb/duckdb#24695](https://github.com/duckdb/duckdb/issues/24695)** states the
underlying problem without prescribing a fix, and is where a different design would land:
`lock_temp_directory` is one shape, and an enforceable `allowed_directories` would solve it
equally well. Watch the issue rather than the PR for whether this fork can be retired.

`0001` and #24694 are the same change and must stay that way. They differ only
where the base version forces it: the `DUCKDB_SETTING_ALIAS` indices, `vfs` versus
`db.config.file_system` in `FileSystem::GetLocal`, and the sqllogictest directory
placeholder. Nothing else should diverge. Note that the build applies neither the `test/`
hunks nor `settings.json` (`NOT_IN_TARBALL` in `build_windmill_patch.rs`), so drift in
those is invisible here and only surfaces when #24694 is rebased.

Until it lands in a released DuckDB the fork is the delivery mechanism, and the exit path
is to delete `crates/libduckdb-sys/windmill/` and go back to the crates.io crate.
