#!/usr/bin/env python3
"""
E2E test for WAC v2 (Workflow-as-Code) with the _executing_key approach.

This test:
1. Creates a preview job with a WAC v2 bun script
2. Waits for the parent to suspend (dispatch child jobs)
3. Waits for child jobs to complete
4. Waits for parent to unsuspend and complete
5. Checks the final result

Usage:
  python3 typescript-client/tests/e2e_wac.py
"""
import json
import sys
import time
import urllib.request

BASE = "http://localhost:8000"
TOKEN = ""  # Will be fetched
WORKSPACE = "admins"

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
            except:
                return raw
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"HTTP {e.code} {method} {path}: {body[:500]}")
        raise

def login():
    global TOKEN
    TOKEN = "PdxixPjjfx05H8xJ8kWAll4RtiLGcfXW"
    # Verify token works
    user = api("GET", "/users/whoami")
    print(f"Logged in as: {user.get('email', 'unknown')}")

def run_preview(code, language="bun"):
    """Run a preview job and return the job ID."""
    result = api("POST", f"/w/{WORKSPACE}/jobs/run/preview", {
        "content": code,
        "language": language,
        "args": {"n": 10},
    })
    print(f"Preview job created: {result}")
    return result

def get_job(job_id):
    return api("GET", f"/w/{WORKSPACE}/jobs_u/get/{job_id}")

def get_result(job_id):
    return api("GET", f"/w/{WORKSPACE}/jobs_u/completed/get_result/{job_id}")

def wait_for_job(job_id, timeout=60, check_interval=2):
    """Wait for a job to complete. Returns the job object."""
    start = time.time()
    while time.time() - start < timeout:
        job = get_job(job_id)
        job_type = job.get("type", "")
        if job_type == "CompletedJob":
            return job
        # Print status
        suspend = job.get("suspend", 0)
        status = "suspended" if suspend and suspend > 0 else "running"
        print(f"  Job {job_id[:8]}... status={status} suspend={suspend} ({time.time()-start:.0f}s)")
        time.sleep(check_interval)
    raise TimeoutError(f"Job {job_id} did not complete within {timeout}s")


WAC_SCRIPT = '''
import { task, workflow } from "windmill-client";

const double = task(async function double(x: number): Promise<number> {
    return x * 2;
});

const add_one = task(async function add_one(x: number): Promise<number> {
    return x + 1;
});

export default workflow(async function main(n: number) {
    const doubled = await double(n);
    const result = await add_one(doubled);
    return { doubled, result };
});
'''

def main():
    print("=== WAC v2 E2E Test ===\n")

    # 1. Login
    login()

    # 2. Run the WAC preview
    print(f"\nRunning WAC v2 preview...")
    job_id = run_preview(WAC_SCRIPT)

    # 3. Wait for completion
    print(f"\nWaiting for job {job_id} to complete...")
    job = wait_for_job(job_id, timeout=120)

    success = job.get("success", False)
    result = job.get("result")

    print(f"\nJob completed! success={success}")
    print(f"Result: {json.dumps(result, indent=2)}")

    if not success:
        print("\nFAILED: Job did not succeed")
        # Print logs if available
        logs = job.get("logs", "")
        if logs:
            print(f"\nLogs:\n{logs}")
        sys.exit(1)

    # 4. Verify result
    expected = {"doubled": 20, "result": 21}
    if result == expected:
        print(f"\nSUCCESS: Sequential workflow result matches expected {expected}")
    else:
        print(f"\nFAILED: Expected {expected}, got {result}")
        sys.exit(1)

    # 5. Test parallel workflow
    print("\n\n=== Parallel Workflow Test ===\n")
    parallel_job_id = run_preview(PARALLEL_WAC_SCRIPT)
    print(f"\nWaiting for parallel job {parallel_job_id} to complete...")
    parallel_job = wait_for_job(parallel_job_id, timeout=120)

    p_success = parallel_job.get("success", False)
    p_result = parallel_job.get("result")

    print(f"\nParallel job completed! success={p_success}")
    print(f"Result: {json.dumps(p_result, indent=2)}")

    if not p_success:
        print("\nFAILED: Parallel job did not succeed")
        sys.exit(1)

    p_expected = {"doubled": 20, "incremented": 11, "combined": 31}
    if p_result == p_expected:
        print(f"\nSUCCESS: Parallel workflow result matches expected {p_expected}")
    else:
        print(f"\nFAILED: Expected {p_expected}, got {p_result}")
        sys.exit(1)

    print("\n\n=== ALL TESTS PASSED ===")

PARALLEL_WAC_SCRIPT = '''
import { task, workflow } from "windmill-client";

const double = task(async function double(x: number): Promise<number> {
    return x * 2;
});

const increment = task(async function increment(x: number): Promise<number> {
    return x + 1;
});

export default workflow(async function main(n: number) {
    const [doubled, incremented] = await Promise.all([
        double(n),
        increment(n),
    ]);
    return { doubled, incremented, combined: doubled + incremented };
});
'''

if __name__ == "__main__":
    main()
