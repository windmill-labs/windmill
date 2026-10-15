#!/usr/bin/env python3
"""flow.py <reps> : a 3-step flow whose DuckDB steps run on each variant's tag; wall p50/p95 and the
sum of the steps' exec time."""
import json, os, subprocess, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run import call
from bench import VARIANTS, pct, db_now, DB

reps = int(sys.argv[1])


def flow(tag):
    step = lambda i, sql: {"id": f"s{i}", "value": {"type": "rawscript", "language": "duckdb", "content": sql,
                                                    "input_transforms": {}, "tag": tag}}
    return {"value": {"modules": [
        step(1, "SELECT 1 AS x;"),
        step(2, "SELECT count(*) AS n FROM range(1000000);"),
        step(3, "SELECT i, i * 2 AS j FROM range(1000) t(i);"),
    ]}, "args": {}}


res = {v: {"wall": [], "steps": []} for v in VARIANTS}
for r in range(reps + 1):
    for v in VARIANTS:
        since = db_now()
        t = time.perf_counter()
        st, out = call("/jobs/run_wait_result/preview_flow", flow(v))
        wall = (time.perf_counter() - t) * 1000
        assert st == 200, out[:300]
        steps = subprocess.run(["psql", DB, "-tAc",
                                f"SELECT coalesce(sum(c.duration_ms),0) FROM v2_job j JOIN v2_job_completed c ON c.id=j.id "
                                f"WHERE j.tag='{v}' AND j.created_at >= '{since}'"], capture_output=True, text=True).stdout.strip()
        if r > 0:  # first round is warm-up
            res[v]["wall"].append(wall)
            res[v]["steps"].append(int(steps))
print(json.dumps({v: {"wall_p50": pct(d["wall"], .5), "wall_p95": pct(d["wall"], .95),
                      "steps_exec_p50": pct(d["steps"], .5), "steps_exec_p95": pct(d["steps"], .95)}
                  for v, d in res.items()}, indent=1))
