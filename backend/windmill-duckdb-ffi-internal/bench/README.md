# DuckDB in-process vs runner: benchmark harness

The scripts behind the numbers in the PR that moved DuckDB jobs into `windmill_duckdb_runner`.
They are scaffolding, not tests. Every measurement runs real preview jobs through release-built
workers, and each worker has exactly one tag, naming the build and the isolation level:

| tag | build | worker env |
|---|---|---|
| `old_nsjail` | main: in-process FFI + SQL fence | `DISABLE_NSJAIL=false` |
| `old_unshare` | main | `DISABLE_NSJAIL=true ENABLE_UNSHARE_PID=true` |
| `old_none` | main: in-process, no fence | `DISABLE_NSJAIL=true` |
| `new_nsjail` | this branch: runner inside nsjail | `DISABLE_NSJAIL=false` |
| `new_unshare` | this branch: runner under unshare | `DISABLE_NSJAIL=true ENABLE_UNSHARE_PID=true` |
| `new_none` | this branch: runner as a plain child | `DISABLE_NSJAIL=true` |

## Setup

```bash
# 1. two release builds with DuckDB + EE: main (OLD, with its libwindmill_duckdb_ffi_internal.so
#    next to `windmill`) and this branch (NEW, with windmill_duckdb_runner next to `windmill`)
(cd backend && RUSTUP_TOOLCHAIN=1.97 cargo build --release \
   --features quickjs,duckdb,enterprise,private,license,parquet)
(cd backend/windmill-duckdb-ffi-internal && CARGO_TARGET_DIR=$PWD/target ./build_dev.sh)

# 2. a release API server for the harness to talk to, so it is not restarted under the run
export DATABASE_URL=postgres://...  WM_BASE=http://localhost:8170  WM_TOKEN=<admin token>
MODE=server PORT=8170 LD_LIBRARY_PATH=$NEW $NEW/windmill &

# 3. workers (MAX_SQL_RESULT_SIZE=256MB, so the guard row has a fixed limit)
bash start_old_vs_new.sh $OLD $NEW 1           # 1 worker per tag
bash worker.sh $NEW flow 8199 &                 # release flow worker for flow.py
```

DuckLake rows need a workspace ducklake named `main` (Postgres catalog, S3 storage; MinIO works.
Release builds refuse FilesystemStorage). They also need `dl.bench`, created by `sql/ducklake.sql`.
`postgres_attach` reads `PG_ATTACH_DSN` (e.g. `host=127.0.0.1 dbname=... user=... password=...`).

## Commands

```bash
VARIANTS=old_nsjail,new_nsjail,old_unshare,new_unshare,old_none,new_none \
  python3 bench.py results_final.json select_1 postgres_attach ducklake_read agg_10m_in_memory \
  result_1k_rows result_100k_rows result_1m_rows spill_15m_sort_256mb
bash start_old_vs_new.sh $OLD $NEW 4            # then, one tag at a time:
python3 throughput.py <tag> 400
python3 cancel.py <tag> <worker pid>             # cancel latency + CPU left burning
python3 memwatch.py <tag> <worker pid> sql/memhog.sql
```

`results/` holds the raw output of the final run (one machine: 24 cores, 60 GB, Linux 7.1,
DuckDB 1.5.5, load average under 4 during the sequential run). `results/prototype/` holds the
earlier runs of the opt-in prototype (`DUCKDB_JAILED_RUNNER`, fence on/off/allowlist), whose
switches no longer exist. `micro.sh` decomposes the runner's cost outside any worker; its
"fenced" case sets `lock_temp_directory`, which only the old patched engine accepts.
