#!/usr/bin/env python3
"""
E2E test for WAC v1 (Workflow-as-Code) — HTTP-dispatch mode.

WAC v1 scripts use @task / task() but NOT @workflow / workflow().
Tasks dispatch via HTTP POST to /jobs/run/workflow_as_code/{job_id}/{task_name}.

This verifies that v1 still works after the v2 client changes.

Usage:
  python3 typescript-client/tests/e2e_wac_v1.py
"""
import json
import sys
import time
import urllib.request

BASE = "http://localhost:8000"
TOKEN = ""
WORKSPACE = "dev"


def api(method, path, data=None):
    url = f"{BASE}/api{path}"
    headers = {"Content-Type": "application/json"}
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            raw = resp.read().decode()
            try:
                return json.loads(raw, strict=False)
            except Exception:
                return raw
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"HTTP {e.code} {method} {path}: {body[:500]}")
        raise


def login():
    global TOKEN
    resp = api("POST", "/auth/login", {"email": "admin@windmill.dev", "password": "changeme"})
    TOKEN = resp
    user = api("GET", "/users/whoami")
    print(f"Logged in as: {user.get('email', 'unknown')}")


def run_preview(code, language="bun", args=None):
    result = api("POST", f"/w/{WORKSPACE}/jobs/run/preview", {
        "content": code,
        "language": language,
        "args": args or {},
    })
    print(f"  Preview job created: {result}")
    return result


def get_job(job_id):
    return api("GET", f"/w/{WORKSPACE}/jobs_u/get/{job_id}")


def wait_for_job(job_id, timeout=120, check_interval=2):
    start = time.time()
    while time.time() - start < timeout:
        job = get_job(job_id)
        if job.get("type") == "CompletedJob":
            return job
        elapsed = time.time() - start
        print(f"    {job_id[:8]}... waiting ({elapsed:.0f}s)")
        time.sleep(check_interval)
    raise TimeoutError(f"Job {job_id} did not complete within {timeout}s")


def check_result(job, expected, label):
    success = job.get("success", False)
    result = job.get("result")
    print(f"  success={success} result={json.dumps(result)}")
    if not success:
        logs = job.get("logs", "")
        print(f"  FAILED: job did not succeed\n  Logs:\n{logs}")
        sys.exit(1)
    if result != expected:
        print(f"  FAILED [{label}]: expected {expected}, got {result}")
        sys.exit(1)
    print(f"  PASSED [{label}]")


# ---------------------------------------------------------------------------
# WAC v1 TypeScript — no workflow() wrapper, tasks dispatch via HTTP
# ---------------------------------------------------------------------------
TS_V1_SEQUENTIAL = '''
import { task } from "windmill-client";

export const double = task(async function double(x: number): Promise<number> {
    return x * 2;
});

export const add_one = task(async function add_one(x: number): Promise<number> {
    return x + 1;
});

export async function main(n: number) {
    const doubled = await double(n);
    const result = await add_one(doubled);
    return { doubled, result };
}
'''

TS_V1_MULTI_PARAM = '''
import { task } from "windmill-client";

export const add = task(async function add(a: number, b: number): Promise<number> {
    return a + b;
});

export async function main(x: number) {
    const result = await add(x, 100);
    return { result };
}
'''

# ---------------------------------------------------------------------------
# WAC v1 Python — no @workflow, tasks dispatch via HTTP
# ---------------------------------------------------------------------------
PY_V1_SEQUENTIAL = '''
import wmill

@wmill.task
def double(x: int) -> int:
    return x * 2

@wmill.task
def add_one(x: int) -> int:
    return x + 1

def main(n: int):
    doubled = double(x=n)
    result = add_one(x=doubled)
    return {"doubled": doubled, "result": result}
'''

PY_V1_MULTI_PARAM = '''
import wmill

@wmill.task
def add(a: int, b: int) -> int:
    return a + b

def main(x: int):
    result = add(a=x, b=100)
    return {"result": result}
'''


def main():
    print("=== WAC v1 E2E Tests ===\n")
    login()

    # --- TypeScript v1: sequential ---
    print("\n[1] TypeScript v1 — sequential tasks")
    job_id = run_preview(TS_V1_SEQUENTIAL, "bun", {"n": 10})
    job = wait_for_job(job_id)
    check_result(job, {"doubled": 20, "result": 21}, "ts_v1_sequential")

    # --- TypeScript v1: multi-param ---
    print("\n[2] TypeScript v1 — multi-param task")
    job_id = run_preview(TS_V1_MULTI_PARAM, "bun", {"x": 42})
    job = wait_for_job(job_id)
    check_result(job, {"result": 142}, "ts_v1_multi_param")

    # --- Python v1: sequential ---
    print("\n[3] Python v1 — sequential tasks")
    job_id = run_preview(PY_V1_SEQUENTIAL, "python3", {"n": 10})
    job = wait_for_job(job_id)
    check_result(job, {"doubled": 20, "result": 21}, "py_v1_sequential")

    # --- Python v1: multi-param ---
    print("\n[4] Python v1 — multi-param task")
    job_id = run_preview(PY_V1_MULTI_PARAM, "python3", {"x": 42})
    job = wait_for_job(job_id)
    check_result(job, {"result": 142}, "py_v1_multi_param")

    print("\n\n=== ALL WAC v1 TESTS PASSED ===")


if __name__ == "__main__":
    main()
