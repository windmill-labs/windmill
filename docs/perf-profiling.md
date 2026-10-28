# Profiling the per-job path

How to find and prove a performance improvement on the job path, which is what any run of a
script or flow pays: push, pull, execution, log writes, completion. The method finds candidates
by measuring rather than by reading code, and keeps only changes whose effect was measured
before and after. A candidate that cannot be measured is not a finding.

## The harness

`backend/tests/perf_profile.rs` runs real jobs through the real queue, worker and database, all
in the test process. Its tests are `#[ignore]`d, so CI only compiles them. For each workload it
reports the following, separately for the idle, push and worker phases:

- requested allocation bytes and allocation count per job (raw, and with the idle rate subtracted)
- allocation sites, sampled once every 128 KiB allocated and once every 2048 allocations. Each
  site is shown as the deepest Windmill `file:line` plus the library call it allocated in
  (`LEAF`), and as the call chain of the four deepest Windmill frames (`CHAIN`)
- with `PERF_SQL=1`, instead of allocations: every SQL statement, how many times it runs per job,
  and its database time per job

Workloads: a deployed bash script, a deployed Bun script, a deployed 3-step identity flow and a
deployed 3-step bash flow. To profile another path, add a workload with the same shape: create
the script or flow with SQL, then call `profile_workload`. Typical additions are a Python
script, a script with many args, a flow with a branch or a loop, or a job with a large result.

```bash
cd backend && source ../.env.local          # DATABASE_URL: sqlx::test creates its DBs from it
OUT=/tmp/perf && mkdir -p $OUT
# allocations and sites
PERF_OUT=$OUT PERF_N=30 cargo test --features quickjs --test perf_profile -- --ignored --nocapture --test-threads=1
# SQL statements per job (a separate run, see the caveats below)
PERF_SQL=1 PERF_OUT=$OUT/sql PERF_N=30 cargo test --features quickjs --test perf_profile -- --ignored --nocapture --test-threads=1
grep -h '^===' $OUT/*.txt                   # the per-job totals, one line per phase
```

Use the same `--features` as the running dev backend so the build reuses its artifacts. One
workload takes about 20 s.

## Reading the output

- **Bytes per job is the main metric.** It is very repeatable, varying less than 0.1% between
  runs on the same code, so 3 runs before and 3 after a change are enough. Time per job is noisy
  in a debug build and is not a measure of anything.
- **A site that sits on top of every script workload deserves attention first.** Per-job setup
  that costs the same however small the job is matters most, because most jobs are small.
- **Discount `pin<…>` / `Box::pin` sites.** They are async state machines, which are many times
  larger without optimisation. Confirm one in release before acting on it.
- **SQL statements per job is usually the larger lever.** Each statement is a network round trip
  in production. Look for statements repeated per job or per step that read something already
  known (a cached value, a row read earlier in the same job), and for appends that could be
  merged.

## A/B a candidate

1. Run the relevant workload 3 times on `main` and keep the `WORKER` (or `PUSH`) lines.
2. Apply the change, then run it 3 times again.
3. When the cost is CPU in a library, also confirm it in a release build: a small standalone
   crate in `/tmp` that pins the same crate version (see `Cargo.lock`), with a counting global
   allocator and a timing loop. Debug numbers measure allocations reliably, but they do not
   measure CPU.
4. Run the tests that pin the behaviour you touched, and put the before/after table in the PR.

Changes to job completion, the queue pull or the flow state machine are the critical path. Test
them beyond the profiler: the existing suites for that area, concurrency, and older Postgres
versions.

## Caveats

- **Split DWARF.** In dev builds a frame's location is a relative `./windmill-*/src/…:line`, and
  inlined frames have bare names (`poll<…>`). The harness keys sites by location for that reason.
  Do not switch it to keying by symbol name.
- **SQL counting changes the allocation numbers.** Once `sqlx::query` events are enabled, sqlx
  formats a summary for every statement. Never read allocations from a `PERF_SQL=1` run.
- **The sampler's own work is excluded.** `IN_SAMPLER` stops the backtrace's allocations from
  being counted. Keep it if you change how samples are captured, or idle reads tens of MB/s.
- **Deployed Bun scripts need a NULL lock.** `''` fails at run time with "expected //bun.lock
  split pattern".
- **Disk.** The test build shares `target/` with the dev backend, but the first build also
  compiles the test-only dependencies. Check `df -h /` first.
