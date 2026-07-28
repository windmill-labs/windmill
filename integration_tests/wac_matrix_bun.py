"""Behaviour matrix for Workflow-as-Code, run against a live Windmill.

Every case is a real preview job, so this exercises the shipped SDK on a real
worker, through dispatch, checkpoint and replay. `check` reads the completed job
and returns None when the behaviour is as expected, or a string describing the
surprise. Unit tests cover neither the worker nor the SDK the worker installs,
which is where the WAC failure contract kept coming apart (#10366, #10367,
#10368).

    python3 integration_tests/wac_matrix_bun.py --base-url http://localhost:8000

Jobs install the **published** SDK, so a local source change is invisible until
it is injected. See `docs/wac-sdk-e2e.md` for that recipe — including the part
that is easy to get wrong: **restart the workers after injecting**, or half the
jobs keep running the old client and the results alternate run to run.

Run it twice, once against the published SDK and once against the injected one:
a case that passes both ways is not testing what you think it is.
"""

import argparse, json, sys, time, urllib.error, urllib.request

BASE = "http://localhost:8000"
WORKSPACE = "admins"


def api(method, path, data=None, token=None):
    req = urllib.request.Request(
        f"{BASE}/api{path}",
        data=json.dumps(data).encode() if data is not None else None,
        headers={"Content-Type": "application/json", **({"Authorization": f"Bearer {token}"} if token else {})},
        method=method,
    )
    try:
        with urllib.request.urlopen(req) as r:
            raw = r.read().decode()
            try:
                return json.loads(raw, strict=False)
            except Exception:
                return raw
    except urllib.error.HTTPError as e:
        return {"__http_error": e.code, "body": e.read().decode()[:300]}


TOKEN = None


def run(script, lang, timeout=300):
    job = api("POST", f"/w/{WORKSPACE}/jobs/run/preview",
              {"content": script, "language": lang, "args": {}}, token=TOKEN)
    if isinstance(job, dict):
        return {"error": job}
    for _ in range(timeout // 2):
        j = api("GET", f"/w/{WORKSPACE}/jobs_u/get/{job}", token=TOKEN)
        if j.get("type") == "CompletedJob":
            return {"success": j.get("success"), "result": j.get("result"), "logs": j.get("logs", "") or "", "id": job}
        time.sleep(2)
    return {"timeout": True, "id": job}

PRELUDE = 'import { task, workflow, step, sleep, parallel } from "windmill-client";\n'
# a body that catches must not swallow the SDK's own control flow
RETHROW = 'const rt = (e) => { if (e?.name !== "TaskError") throw e; };\n'

CASES = []


def case(name, script, check, lang="bun", args=None):
    CASES.append({"name": name, "script": script, "check": check, "lang": lang, "args": args})


# ---------- happy paths ----------
case(
    "sequential tasks",
    PRELUDE + '''
const dbl = task(async function dbl(x){ return x*2; });
const inc = task(async function inc(x){ return x+1; });
export default workflow(async function main(){ const a = await dbl(20); return { out: await inc(a) }; });
''',
    lambda j: None if j["result"] == {"out": 41} else f"got {j['result']}",
)

case(
    "a step's rich return type is the same in both rounds",
    PRELUDE + '''
const dbl = task(async function dbl(x){ return x*2; });
export default workflow(async function main(){
  const d = await step("d", () => new Date("2026-01-01T00:00:00Z"));
  const m = await step("m", () => [1,2]);
  const seen = await step("seen", () => ({ d: typeof d, isDate: d instanceof Date, m: Array.isArray(m) }));
  await dbl(1);
  return { firstRound: seen, laterRound: { d: typeof d, isDate: d instanceof Date, m: Array.isArray(m) } };
});
''',
    lambda j: None
    if j["result"]["firstRound"] == j["result"]["laterRound"]
    else f"rounds disagree: {j['result']}",
)

# ---------- failure record ----------
case(
    "a caught step failure and task failure read alike",
    PRELUDE + RETHROW + '''
const boom = task(async function boom(){ const e = new Error("nope"); e.name = "HttpError"; e.code = 429; throw e; });
export default workflow(async function main(){
  let s, t;
  try { await step("s", () => { const e = new Error("nope"); e.name = "HttpError"; e.code = 429; throw e; }); }
  catch (e) { rt(e); s = { name: e.result?.error?.name, message: e.result?.error?.message, extra: e.result?.error?.extra, child: e.child_job_id ?? null, cause: e.cause ?? null }; }
  try { await boom(); }
  catch (e) { rt(e); t = { name: e.result?.error?.name, message: e.result?.error?.message, extra: e.result?.error?.extra, child: !!e.child_job_id, cause: e.cause ?? null }; }
  return { s, t };
});
''',
    lambda j: None
    if (
        j["result"]["s"]["name"] == j["result"]["t"]["name"] == "HttpError"
        and j["result"]["s"]["message"] == j["result"]["t"]["message"] == "nope"
        and j["result"]["s"]["extra"] == j["result"]["t"]["extra"] == {"code": 429}
        and j["result"]["s"]["child"] is None
        and j["result"]["t"]["child"] is True
        and j["result"]["s"]["cause"] is None
    )
    else f"records differ: {json.dumps(j['result'])}",
)

case(
    "a non-Error throw still produces a usable record",
    PRELUDE + RETHROW + '''
export default workflow(async function main(){
  let o, s;
  try { await step("o", () => { throw { name: "Thrown", message: "boom", code: 1 }; }); }
  catch (e) { rt(e); o = e.result?.error; }
  try { await step("s", () => { throw "plain"; }); }
  catch (e) { rt(e); s = e.result?.error; }
  return { o, s };
});
''',
    lambda j: None
    if (
        j["result"]["o"]["name"] == "Thrown"
        and j["result"]["o"]["message"] == "boom"
        and j["result"]["o"]["extra"] == {"code": 1}
        and j["result"]["s"]["name"] == "Error"
    )
    else f"got {json.dumps(j['result'])}",
)

case(
    "a circular property does not take the workflow down",
    PRELUDE + RETHROW + '''
export default workflow(async function main(){
  let r;
  try {
    await step("s", () => {
      const e = new Error("socket hang up");
      e.code = "ECONNRESET";
      const req = { url: "/x" }; req.self = req; e.request = req;
      throw e;
    });
  } catch (e) { rt(e); r = e.result?.error; }
  return { name: r.name, extra: r.extra };
});
''',
    lambda j: None
    if j["result"]["extra"] == {"code": "ECONNRESET"}
    else f"got {json.dumps(j['result'])}",
)

case(
    "an oversized stack is bounded but the step still completes",
    PRELUDE + RETHROW + '''
export default workflow(async function main(){
  let r;
  try {
    await step("s", () => { const e = new Error("big"); e.stack = "x".repeat(200000); throw e; });
  } catch (e) { rt(e); r = e.result?.error; }
  return { len: (r.stack ?? "").length, truncated: (r.stack ?? "").endsWith("... (truncated)") };
});
''',
    lambda j: None
    if j["result"]["truncated"] and j["result"]["len"] < 200000
    else f"got {json.dumps(j['result'])}",
)

case(
    "an oversized extra is dropped and flagged",
    PRELUDE + RETHROW + '''
export default workflow(async function main(){
  let r;
  try {
    await step("s", () => { const e = new Error("big"); e.body = "y".repeat(200000); throw e; });
  } catch (e) { rt(e); r = e.result?.error; }
  return { hasExtra: r.extra !== undefined, omitted: r.extra_omitted ?? false };
});
''',
    lambda j: None
    if j["result"]["omitted"] is True and j["result"]["hasExtra"] is False
    else f"got {json.dumps(j['result'])}",
)

# ---------- control flow around failures ----------
case(
    "a task failure caught by a broad catch is reported, not swallowed",
    PRELUDE + '''
const boom = task(async function boom(){ throw new TypeError("nope"); });
export default workflow(async function main(){
  try { await boom(); } catch (e) { return { caught: e?.name ?? "unnamed", isRecord: !!e?.result }; }
  return { caught: null };
});
''',
    lambda j: None
    if (j["result"] or {}).get("caught") == "TaskError"
    else f"got {j['result']} (a broad catch must still see the failure, cf #10366)",
)

def main():
    global BASE, WORKSPACE, TOKEN
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--base-url", default=BASE)
    p.add_argument("--workspace", default=WORKSPACE)
    p.add_argument("--email", default="admin@windmill.dev")
    p.add_argument("--password", default="changeme")
    a = p.parse_args()
    BASE, WORKSPACE = a.base_url, a.workspace
    TOKEN = api("POST", "/auth/login", {"email": a.email, "password": a.password})
    if not isinstance(TOKEN, str):
        sys.exit(f"could not log in to {BASE}: {TOKEN}")

    width = max(len(c["name"]) for c in CASES)
    failures = 0
    for c in CASES:
        j = run(c["script"], lang="bun")
        if j.get("timeout"):
            verdict, detail = "TIMEOUT", j.get("id", "")
        elif "error" in j:
            verdict, detail = "ERROR", str(j["error"])[:120]
        else:
            try:
                problem = c["check"](j)
            except Exception as e:
                problem = f"check raised {type(e).__name__}: {e} | result={json.dumps(j.get('result'))[:200]}"
            if problem is None:
                verdict, detail = "ok", ""
            else:
                verdict, detail = "SURPRISE", problem
        if verdict != "ok":
            failures += 1
        print(f"{c['name']:<{width}}  {verdict:<9} {detail}", flush=True)
    print(f"\n{len(CASES) - failures}/{len(CASES)} behaved as expected")


if __name__ == "__main__":
    main()
