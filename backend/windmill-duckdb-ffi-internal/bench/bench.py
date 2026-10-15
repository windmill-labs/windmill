#!/usr/bin/env python3
"""Benchmark DuckDB jobs across worker variants (in-process FFI vs out-of-process runner).

Every job is a real preview job (`run_wait_result/preview`) served by a release worker whose
only tag is the variant name. Variants are interleaved per repetition so drift (thermal, page
cache, the network) spreads evenly. Two numbers per job:
  wall  = client-side round trip (submit -> result), includes queue pickup
  exec  = v2_job_completed.duration_ms (started -> completed on the worker)

usage: bench.py <out.json> [workload ...]
"""
import json, os, statistics, subprocess, sys, time, uuid

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run import call, run_wait

DB = os.environ["DATABASE_URL"]
VARIANTS = os.environ.get("VARIANTS", "bench_inproc,bench_proc,bench_jail,bench_nofence").split(",")
PARQUET = "https://blobs.duckdb.org/data/taxi_2019_04.parquet"

PG = """INSTALL postgres;
LOAD postgres;
ATTACH '""" + os.environ["PG_ATTACH_DSN"] + """' AS pg (TYPE postgres, READ_ONLY);
SELECT count(*) AS n, count(DISTINCT kind) AS kinds FROM pg.public.v2_job;"""

SPILL = """SET memory_limit='256MB';
SET threads=4;
SET preserve_insertion_order=false;
SELECT count(*) AS n, max(s) AS mx FROM (SELECT i, md5(i::VARCHAR) || md5((i+1)::VARCHAR) AS s FROM range(15000000) t(i) ORDER BY s);"""


def rows(n):
    return f"SELECT i AS id, i * 2 AS twice, 'row-' || i::VARCHAR AS label FROM range({n}) t(i);"


WORKLOADS = {
    # name: (sql, reps)
    "select_1": ("SELECT 1 AS x;", 30),
    "httpfs_parquet_count": (f"SELECT count(*) AS n FROM read_parquet('{PARQUET}');", 10),
    "httpfs_parquet_scan": (
        f"SELECT avg(trip_distance) AS d, sum(total_amount) AS t FROM read_parquet('{PARQUET}');",
        5,
    ),
    "ducklake_read": (
        "ATTACH 'ducklake://main' AS dl;\nSELECT count(*) AS n, max(s) AS mx FROM dl.bench;",
        10,
    ),
    "ducklake_write_100k": (
        "ATTACH 'ducklake://main' AS dl;\nCREATE TABLE IF NOT EXISTS dl.w (id BIGINT, s VARCHAR);\n"
        "INSERT INTO dl.w SELECT i, md5(i::VARCHAR) FROM range(100000) t(i);\nSELECT 1 AS ok;",
        5,
    ),
    "postgres_attach": (PG, 10),
    "agg_10m_in_memory": (
        "SELECT count(*) AS groups, sum(s) AS total FROM (SELECT i % 1000003 AS k, sum(i) AS s "
        "FROM range(10000000) t(i) GROUP BY k);",
        10,
    ),
    "spill_15m_sort_256mb": (SPILL, 5),
    "result_1k_rows": (rows(1_000), 20),
    "result_100k_rows": (rows(100_000), 10),
    "result_1m_rows": (rows(1_000_000), 5),
    # Past MAX_SQL_RESULT_SIZE=256MB on the bench workers: both paths must refuse it the same way.
    "result_over_guard_10m_rows": (rows(10_000_000), 3),
}


def exec_ms(tag, since):
    out = subprocess.run(
        ["psql", DB, "-tAc",
         f"SELECT c.duration_ms FROM v2_job j JOIN v2_job_completed c ON c.id = j.id "
         f"WHERE j.tag = '{tag}' AND j.created_at >= '{since}' ORDER BY j.created_at DESC LIMIT 1"],
        capture_output=True, text=True,
    ).stdout.strip()
    return int(out) if out else None


def db_now():
    return subprocess.run(["psql", DB, "-tAc", "SELECT now()"], capture_output=True, text=True).stdout.strip()


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return None
    k = (len(xs) - 1) * p
    lo, hi = int(k), min(int(k) + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def summarize(samples):
    ok = [s for s in samples if s["status"] == 200]
    wall = [s["wall_ms"] for s in samples]
    ex = [s["exec_ms"] for s in samples if s["exec_ms"] is not None]
    return {
        "n": len(samples),
        "ok": len(ok),
        "wall_p50": pct(wall, 0.5), "wall_p95": pct(wall, 0.95),
        "exec_p50": pct(ex, 0.5), "exec_p95": pct(ex, 0.95),
        "first_error": next((s["out"][:300] for s in samples if s["status"] != 200), None),
    }


def main():
    out_path = sys.argv[1]
    names = sys.argv[2:] or list(WORKLOADS)
    results = json.load(open(out_path)) if os.path.exists(out_path) else {}
    for name in names:
        sql, reps = WORKLOADS[name]
        # one warm-up per variant (extension download, page cache, first dlopen)
        for v in VARIANTS:
            run_wait(v, sql)
        samples = {v: [] for v in VARIANTS}
        for r in range(reps):
            for v in VARIANTS:
                since = db_now()
                st, out, dt = run_wait(v, sql)
                samples[v].append({"status": st, "wall_ms": dt * 1000, "exec_ms": exec_ms(v, since), "out": out})
        results[name] = {v: summarize(s) for v, s in samples.items()}
        line = " | ".join(
            f"{v.replace('bench_', '')}: {results[name][v]['exec_p50']}/{results[name][v]['exec_p95']}ms ok={results[name][v]['ok']}"
            for v in VARIANTS)
        print(f"{name}: {line}", flush=True)
        json.dump(results, open(out_path, "w"), indent=1)


if __name__ == "__main__":
    main()
