# How DuckDB jobs run

A DuckDB job runs in `windmill_duckdb_runner`, a binary built from
`backend/windmill-duckdb-ffi-internal` (`src/bin`) and shipped next to the `windmill` binary.
The worker spawns one runner per DuckDB connection a job opens: the job itself, and the
`on_schema_change=sync` pre-pass. It never loads the engine into its own process.

The runner gets the same job isolation as every other language. `duckdb_runner::runner_mode`
decides it per job:

| worker isolates with | runner runs | what it can see |
|---|---|---|
| nsjail | inside `run.duckdb.config.proto` | read-only `/usr`, `/lib`, `/etc`; its own `/tmp`; its job dir read-only, except `duckdb/` (spill + result); a read-only extension cache |
| unshare | under `build_command_with_isolation` | the worker user's file system, its own PID namespace, a cleared environment |
| nothing | as a plain child process | the worker user's file system, a cleared environment |

There is no SQL-level policy on top. A script can read and write what the jail lets it and load
any extension DuckDB will sign, as a Python job can read the same files and load native code.
Under nsjail its disk use is capped from the worker (below). Network egress is open in every
mode, as it is for the other languages: the S3 proxy, metadata endpoints and internal hosts are
all reachable.

## What crosses to the runner

`spawn_runner` writes one JSON request to the runner's stdin, so nothing secret is put on argv or
into the environment:

- the blocks, already rewritten by the worker: resource `ATTACH`es, DuckLake and datatable
  attaches with their credentials inlined, workspace macros;
- the job args, the job token, `base_internal_url` and the workspace id. The runner derives the
  S3/GCS secrets for the workspace's S3 proxy from the token, which it reaches over the network;
- `memory_limit` (`DUCKDB_MEMORY_LIMIT`, or 80% of the worker's cgroup), the temp directory (the
  job dir), and the result-size cap (`MAX_SQL_RESULT_SIZE`).

The environment is cleared except `PATH`, `HOME` (outside nsjail), `DUCKDB_MAX_TEMP_DIRECTORY_SIZE`,
`GOOGLE_APPLICATION_CREDENTIALS` (BigQuery `ATTACH`, whose credentials file nsjail mounts) and the
proxy variables. In particular a script cannot use the worker's own `AWS_*` credential chain.

Under nsjail the job dir is mounted read-only at its host path, so every path the worker wrote
into the blocks (Postgres root certificates) resolves unchanged. The one writable place on the
host is `{job dir}/duckdb/`: the spill directory and the result file, which the worker opens
without following symlinks, since the script can write there.

The runner writes its answer to a file in that directory: a first line with the last statement's
column order, then either the collected results or `ERROR <json-encoded message>`.
`finish_run` / `finish_prepare` in `duckdb_executor.rs` turn that into the job result.

## Failure, cancellation and memory

- **A crash fails the job, never the worker.** A runner that exits without writing its result
  becomes `The DuckDB engine process … The worker is unaffected.` with the tail of its stderr.
  An engine `std::terminate` (for example a remote `temp_directory` that has to spill) and a
  panic in result rendering both land here.
- **Cancellation and timeouts kill the engine.** The executor drops the job future, and
  `KillGroupOnDrop` kills the runner's whole process group. Killing the spawned process alone is
  not enough: `unshare --fork` leaves its child running.
- **The kernel OOM-kills the runner, not the worker.** `JOB_OOM_SCORE_ADJ` is written in
  `pre_exec`, before nsjail or `unshare` is exec'd, so the runner inherits it. Written after spawn
  it would race their fork, and the jail's `/proc` is read-only. Within the job, DuckDB bounds
  itself through `memory_limit` and spills past it.
- **Runner output is bounded.** stdout/stderr are drained keeping only the last 4 KB: a script
  controls how much the engine prints (`CALL enable_logging(storage='stdout')`).

## Extensions

Outside nsjail the runner uses the worker user's `~/.duckdb/extensions` and installs whatever a
script names.

Under nsjail it gets its own cache, `~/.duckdb/windmill_jail_extensions`, mounted read-only.
Only the worker writes to it: before a jailed run it starts the runner unjailed in `install`
mode, with only `INSTALL` statements it built itself, for `httpfs`, `azure`, `parquet`, `json`
and every extension a block installs by name. That runs no script SQL and loads nothing, so no
extension code runs outside the jail. It happens once per extension per worker process, except
for `FORCE INSTALL`, which is honored every time and becomes a plain `INSTALL` inside the jail.
A job can therefore neither add to nor replace what the next job loads from that cache. An
extension used without being named and outside those four (`icu` behind `SET TimeZone`, say)
fails on the read-only mount: name it with `INSTALL`.

A script can still point `extension_directory` somewhere writable and install into it. That only
affects its own connection, and is no more than a Python job could do.

## The engine

The runner links duckdb-rs from crates.io, pinned with `=` so `cargo update` cannot move the
engine. The engine loads the **prebuilt** extensions from `extensions.duckdb.org`, compiled for the
exact release it reports, so an engine bump is a version change in `Cargo.toml` and nothing else.

Confinement belongs to the job isolation, not to engine settings. DuckDB spills through the same
file system a query reads with, so `disabled_filesystems='LocalFileSystem'` also stops every query
that outgrows `memory_limit`, and no combination of stock settings gives "spill, but no other local
file access". Fencing the engine at the SQL level again would need a patched engine to keep
spilling.

## Bounding disk use

The cap is `DUCKDB_MAX_TEMP_DIRECTORY_SIZE`, or `6GiB` on a `CLOUD_HOSTED` worker that does not
set it. The cloud runs one worker per pod on nodes with no per-pod ephemeral-storage limit, and a
worker pod already holds several GiB of runtime caches, so the cap is sized against a mostly full
node; re-measure before raising it. Unset elsewhere, DuckDB's own default applies: 90% of the
temp volume's free space.

It is applied twice:

- **DuckDB's `max_temp_directory_size`**, set on every connection, so an ordinary query that
  outgrows it fails with the engine's `failed to offload data block … max_temp_directory_size`
  error and cleans up its temp files.
- **A worker-side watchdog**, under nsjail. A script can raise the engine setting, so on its own
  it is only a default. The worker sums the allocated size of `{job dir}/duckdb/` once a second
  and kills the runner when it passes the cap, whatever wrote the bytes: spill, `COPY … TO`, or
  the result file. With a disk-backed jail `/tmp` (`NSJAIL_TMP_BACKING=disk`) that `/tmp` lives
inside the same directory, so it counts too; a RAM-backed one is bounded by its tmpfs size.
Enforcement lags by up to a second of writing. Outside nsjail the script can
  write anywhere the worker user can, as other languages can, so there the cap stays a default.

The value is DuckDB's spelling, not Kubernetes': `KiB`/`MiB`/`GiB`/`TiB` for 1024-based units,
`KB`/`MB`/`GB`/`TB` for 1000-based. An unparseable value fails every DuckDB job on the worker
rather than being ignored. `DUCKDB_MEMORY_LIMIT` is spelled the same way, and the two are set in
separate statements so a parse error names the right variable.
