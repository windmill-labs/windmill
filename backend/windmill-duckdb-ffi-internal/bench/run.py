#!/usr/bin/env python3
"""run.py <tag> <sql-file-or-literal> : run one DuckDB preview job on a tag and print the result."""
import json, sys, time, urllib.request, os

BASE = os.environ.get("WM_BASE", "http://localhost:8000") + "/api/w/" + os.environ.get("WM_WORKSPACE", "admins")
TOKEN = os.environ["WM_TOKEN"]


def call(path, body=None, method=None):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"},
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=600) as r:
            return r.status, r.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


def run_wait(tag, sql, args=None):
    t = time.perf_counter()
    st, out = call(
        "/jobs/run_wait_result/preview",
        {"content": sql, "language": "duckdb", "args": args or {}, "tag": tag},
    )
    return st, out, time.perf_counter() - t


if __name__ == "__main__":
    tag, sql = sys.argv[1], sys.argv[2]
    if os.path.exists(sql):
        sql = open(sql).read()
    st, out, dt = run_wait(tag, sql)
    print(st, f"{dt*1000:.0f}ms", out[:3000])
