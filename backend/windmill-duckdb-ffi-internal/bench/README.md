# In-process vs jailed DuckDB: benchmark harness

The scripts behind the numbers in the PR that added `windmill_duckdb_runner`. These are prototype
scaffolding, not tests. Every measurement runs real preview jobs through release-built workers.
Each worker has exactly one tag, which names the variant:

| tag | worker env | what runs |
|---|---|---|
| `bench_inproc` | `DISABLE_NSJAIL=false` | today's isolating worker: in-process FFI + SQL fence |
| `bench_proc` | `+ DUCKDB_JAILED_RUNNER=process` | runner as a plain child process, fence kept (process cost alone) |
| `bench_jail` | `+ DUCKDB_JAILED_RUNNER=true` | runner inside nsjail, fence kept (defense in depth) |
| `bench_nofence` | `+ DUCKDB_JAILED_RUNNER_FENCE=false` | runner inside nsjail, no fence, no `lock_temp_directory` |
| `bench_allow` | `+ DUCKDB_JAILED_RUNNER_FENCE=allowlist` | runner inside nsjail, extension allowlist + `lock_temp_directory`, no `disabled_filesystems` |
| `bench_inproc_nofence` | `DISABLE_DUCKDB_FS_SANDBOX=true` | in-process with no fence (a non-isolating worker's cost) |

## Setup

```bash
# 1. binaries: the FFI cdylib + runner, and a release windmill with DuckDB + EE isolation
(cd backend/windmill-duckdb-ffi-internal && CARGO_TARGET_DIR=$PWD/target ./build_dev.sh)
(cd backend && RUSTUP_TOOLCHAIN=1.97 cargo build --release \
   --features quickjs,duckdb,enterprise,private,license,parquet)
BIN=~/wm-bench-bin && mkdir -p $BIN
cp backend/target/release/windmill $BIN/
cp backend/target/debug/{libwindmill_duckdb_ffi_internal.so,windmill_duckdb_runner} $BIN/

# 2. a release API server for the harness to talk to, so it is not restarted under the run
export DATABASE_URL=postgres://...  WM_BASE=http://localhost:8170  WM_TOKEN=<admin token>
MODE=server PORT=8170 LD_LIBRARY_PATH=$BIN $BIN/windmill &

# 3. workers (MAX_SQL_RESULT_SIZE=256MB, so the guard row has a fixed limit)
bash start_workers.sh $BIN 1                  # 1 worker per variant
bash worker.sh $BIN flow 8199 &               # release flow worker for flow.py
```

DuckLake rows need a workspace ducklake named `main` (Postgres catalog, S3 storage; MinIO works.
Release builds refuse FilesystemStorage). They also need `dl.bench`, created by `sql/ducklake.sql`.
`postgres_attach` reads `PG_ATTACH_DSN` (e.g. `host=127.0.0.1 dbname=... user=... password=...`).

## Commands

```bash
python3 bench.py results_seq.json                       # sequential matrix, all variants interleaved
VARIANTS=bench_inproc_nofence python3 bench.py results_baseline.json select_1 ...
bash micro.sh $BIN 30                                   # cost decomposition, no worker involved
for t in bench_inproc bench_proc bench_jail bench_nofence; do python3 throughput.py $t 200; done
bash start_workers.sh $BIN 4                            # then the same with 400 jobs
VARIANTS=bench_inproc,bench_jail,bench_nofence python3 flow.py 10
bash firstjob.sh $BIN
python3 cancel.py <tag> <worker pid>                    # cancel latency + CPU left burning
python3 memwatch.py <tag> <worker pid> sql/memhog.sql   # worker vs runner RSS
```

`results/` holds the raw output of the run reported in the PR (one machine: 24 cores, 60 GB,
Linux 7.1, DuckDB 1.5.5).
