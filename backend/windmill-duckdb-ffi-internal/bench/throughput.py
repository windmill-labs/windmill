#!/usr/bin/env python3
"""throughput.py <tag> <n_jobs> : enqueue n SELECT 1 DuckDB jobs at once on a tag and measure
jobs/s from the first start to the last completion (all workers serving the tag)."""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run import call

DB = os.environ["DATABASE_URL"]
tag, n = sys.argv[1], int(sys.argv[2])
sql = sys.argv[3] if len(sys.argv) > 3 else "SELECT 1 AS x;"


def submit(_):
    st, jid = call("/jobs/run/preview", {"content": sql, "language": "duckdb", "args": {}, "tag": tag})
    return jid.strip().strip('"')


with ThreadPoolExecutor(16) as ex:
    ids = list(ex.map(submit, range(n)))
idlist = ",".join(f"'{i}'" for i in ids)
while True:
    done = subprocess.run(["psql", DB, "-tAc", f"SELECT count(*) FROM v2_job_completed WHERE id IN ({idlist})"],
                          capture_output=True, text=True).stdout.strip()
    if int(done or 0) >= n:
        break
    time.sleep(0.5)
q = (f"SELECT extract(epoch FROM max(completed_at) - min(started_at)), "
     f"percentile_cont(0.5) WITHIN GROUP (ORDER BY duration_ms), "
     f"percentile_cont(0.95) WITHIN GROUP (ORDER BY duration_ms), "
     f"count(*) FILTER (WHERE status = 'success') "
     f"FROM v2_job_completed WHERE id IN ({idlist})")
span, p50, p95, ok = subprocess.run(["psql", DB, "-tAc", q], capture_output=True, text=True).stdout.strip().split("|")
print(json.dumps({"tag": tag, "jobs": n, "ok": int(ok), "span_s": round(float(span), 2),
                  "jobs_per_s": round(n / float(span), 1), "exec_p50_ms": float(p50), "exec_p95_ms": float(p95)}))
