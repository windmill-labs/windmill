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

## What the patch adds

`lock_temp_directory` (BOOLEAN, default false, global, irreversible). Setting it:

1. fixes `temp_directory` in place — it can no longer be modified, and the setting itself
   cannot be unset or reset;
2. routes the buffer manager's temporary files through a local file system that
   `disabled_filesystems` does not apply to.

The two halves are what make it safe as a pair. The exemption is only sound because the
directory can no longer be redirected: without (1), a script could point `temp_directory`
at an arbitrary path and get a blind write primitive through the exemption.
`allowed_directories` still applies either way.

The isolation transform therefore emits `SET lock_temp_directory=true` **before**
`disabled_filesystems`, while the local file system is still reachable, and after the FFI
has already pointed `temp_directory` at the job dir
(`configure_duckdb_resource_limits`). Everything a script authored runs after all three
settings, so a script can never move the temp directory.

## Where the patched engine comes from

`backend/windmill-duckdb-ffi-internal` depends on
[`windmill-labs/duckdb-rs`](https://github.com/windmill-labs/duckdb-rs), branch
`windmill/duckdb-1.5.5`, pinned by rev. That fork is duckdb-rs 1.10505.0 (DuckDB 1.5.5)
plus one commit; `duckdb.tar.gz` is left byte-identical to upstream's and the patch is
applied to the extracted sources at build time, so `git diff` against `duckdb/duckdb-rs`
shows the whole delta as text. `crates/libduckdb-sys/windmill/README.md` in the fork
carries the rebase recipe.

### The patch may not move a struct member

The engine loads the **prebuilt** extensions from `extensions.duckdb.org` — httpfs, azure,
postgres, ducklake, parquet, and the rest of `EXTENSION_ALLOWLIST` — which are compiled
against the struct layouts of the release the engine claims to be. A patch that shifts a
member those extensions reach breaks them, and not loudly: the extension reads the
neighbouring field and `LOAD httpfs` **blocks forever** rather than failing, so every
DuckDB job hangs until its timeout.

The first revision of this patch declared its new `DatabaseInstance` member next to
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

## What stops the patch disappearing silently

An unpatched engine compiles, links and passes every existing test. Three things catch it:

- **The fork's build script** asserts a marker string is present in the patched sources
  after applying the patch, and panics if the patch no longer applies.
- **`patched_engine_tests` in the FFI crate** runs a query that must spill, first
  without the lock (asserting the fence stops it, which is what proves the query really
  spills) and then with it (asserting it succeeds). It fails outright on a stock engine,
  where the `SET` is an unrecognized parameter. CI runs it via the FFI crate's own
  `cargo test` step in `backend-test.yml`.
- **The transform itself fails closed**: on a stock engine `SET lock_temp_directory=true`
  errors, so every DuckDB job on an isolating worker fails immediately rather than
  quietly losing the ability to spill.

## Upstream

The patch is written against `duckdb/duckdb` `v1.5.5` in the shape upstream takes changes.
Besides the engine sources it carries `src/common/settings.json` (the settings generator's
input, regenerated with `scripts/generate_settings.py` and formatted with clang-format
11.0.1), two sqllogictests under `test/sql/settings/`, and one line in
`test/api/test_reset.cpp` — that test requires every setting to survive a `RESET`, so an
irreversible one has to join the exclusion list `disabled_filesystems` and
`lock_configuration` are already on. It is a `git format-patch` file, so it applies with
`git am`:

```bash
git clone https://github.com/duckdb/duckdb.git && cd duckdb && git checkout v1.5.5
git am < .../crates/libduckdb-sys/windmill/0001-lock-temp-directory.patch
GEN=ninja make && ./build/release/test/unittest "test/sql/settings/lock_temp_directory*"
```

It has **not** been proposed to duckdb/duckdb. Their `CONTRIBUTING.md` asks contributors
not to submit LLM-generated pull requests, and asks outside contributors to discuss the
change on GitHub first and to run CI on a fork before opening one. Filing it is a human
decision; until it lands in a released DuckDB, the fork is the delivery mechanism, and
the exit path is to delete `crates/libduckdb-sys/windmill/` and go back to the crates.io
crate.
