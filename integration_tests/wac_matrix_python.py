"""Behaviour matrix for Workflow-as-Code, run against a live Windmill.

Every case is a real preview job, so this exercises the shipped SDK on a real
worker, through dispatch, checkpoint and replay. `check` reads the completed job
and returns None when the behaviour is as expected, or a string describing the
surprise. Unit tests cover neither the worker nor the SDK the worker installs,
which is where the WAC failure contract kept coming apart (#10366, #10367,
#10368).

    python3 integration_tests/wac_matrix_python.py --base-url http://localhost:8000

Jobs install the **published** SDK, so a local source change is invisible until
it is injected. See `docs/wac-sdk-e2e.md` for that recipe — including the part
that is easy to get wrong: **restart the workers after injecting**, or half the
jobs keep running the old client and the results alternate run to run.

Run it twice, once against the published SDK and once against the injected one:
a case that passes both ways is not testing what you think it is.
"""
# Mirrors wac_matrix_bun.py case for case where python can express the same
# thing. A python body can only raise BaseException subclasses, so the "throw
# a plain object" cases become custom exception classes instead.

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


PRELUDE = "import asyncio\nfrom wmill import task, workflow, step, sleep, TaskError\n"

CASES = []


def case(name, script, check):
    CASES.append({"name": name, "script": script, "check": check})


case(
    "sequential tasks",
    PRELUDE + '''
@task
async def dbl(x: int):
    return x * 2

@task
async def inc(x: int):
    return x + 1

@workflow
async def main():
    a = await dbl(x=20)
    return {"out": await inc(x=a)}
''',
    lambda j: None if j["result"] == {"out": 41} else f"got {j['result']}",
)

case(
    "a step's rich return type is the same in both rounds",
    PRELUDE + '''
from datetime import datetime, timezone

@task
async def dbl(x: int):
    return x * 2

@workflow
async def main():
    d = await step("d", lambda: datetime(2026, 1, 1, tzinfo=timezone.utc))
    t = await step("t", lambda: (1, 2))
    seen = await step("seen", lambda: {"d": type(d).__name__, "t": type(t).__name__})
    await dbl(x=1)
    return {"firstRound": seen, "laterRound": {"d": type(d).__name__, "t": type(t).__name__}}
''',
    lambda j: None
    if j["result"]["firstRound"] == j["result"]["laterRound"]
    else f"rounds disagree: {j['result']}",
)

case(
    "a caught step failure and task failure read alike",
    PRELUDE + '''
class HttpError(Exception):
    def __init__(self):
        super().__init__("nope")
        self.code = 429

@task
async def boom():
    raise HttpError()

def raiser():
    raise HttpError()

@workflow
async def main():
    s = t = None
    try:
        await step("s", raiser)
    except TaskError as e:
        s = {"name": (e.result or {}).get("error", {}).get("name"),
             "message": (e.result or {}).get("error", {}).get("message"),
             "extra": (e.result or {}).get("error", {}).get("extra"),
             "child": e.child_job_id, "cause": repr(e.__cause__)}
    try:
        await boom()
    except TaskError as e:
        t = {"name": (e.result or {}).get("error", {}).get("name"),
             "message": (e.result or {}).get("error", {}).get("message"),
             "extra": (e.result or {}).get("error", {}).get("extra"),
             "child": bool(e.child_job_id), "cause": repr(e.__cause__)}
    return {"s": s, "t": t}
''',
    lambda j: None
    if (
        j["result"]["s"]["name"] == j["result"]["t"]["name"] == "HttpError"
        and j["result"]["s"]["message"] == j["result"]["t"]["message"] == "nope"
        and j["result"]["s"]["extra"] == j["result"]["t"]["extra"] == {"code": 429}
        and j["result"]["s"]["child"] is None
        and j["result"]["t"]["child"] is True
        and j["result"]["s"]["cause"] == "None"
    )
    else f"records differ: {json.dumps(j['result'])}",
)

case(
    "a caught step failure is identical in the live round and the replay",
    PRELUDE + '''
@task
async def dbl(x: int):
    return x * 2

def raiser():
    raise ValueError("nope")

@workflow
async def main():
    live = None
    try:
        await step("s", raiser)
    except TaskError as e:
        live = {"r": e.result, "k": e.step_key, "c": repr(e.__cause__)}
    remembered = await step("remembered", lambda: live)
    await dbl(x=1)
    return {"remembered": remembered, "replay": live}
''',
    lambda j: None
    if j["result"]["remembered"] == j["result"]["replay"]
    else f"live != replay: {json.dumps(j['result'])}",
)

case(
    "a custom exception's attributes reach extra",
    PRELUDE + '''
class Detailed(Exception):
    def __init__(self):
        super().__init__("detail")
        self.code = "ECONNRESET"
        self.attempt = 3

def raiser():
    raise Detailed()

@workflow
async def main():
    try:
        await step("s", raiser)
    except TaskError as e:
        return {"error": (e.result or {}).get("error")}
    return {"error": None}
''',
    lambda j: None
    if (
        j["result"]["error"]["name"] == "Detailed"
        and j["result"]["error"]["extra"] == {"code": "ECONNRESET", "attempt": 3}
    )
    else f"got {json.dumps(j['result'])}",
)

case(
    "an unserializable attribute is dropped on its own",
    PRELUDE + '''
class WithLiveObject(Exception):
    def __init__(self):
        super().__init__("boom")
        self.code = "ECONNRESET"
        self.socket = object()

def raiser():
    raise WithLiveObject()

@workflow
async def main():
    try:
        await step("s", raiser)
    except TaskError as e:
        return {"extra": (e.result or {}).get("error", {}).get("extra")}
    return {"extra": None}
''',
    lambda j: None
    if (j["result"]["extra"] or {}).get("code") == "ECONNRESET"
    else f"got {json.dumps(j['result'])} (a live object must not take the rest of extra with it)",
)

case(
    "an exception whose __str__ raises does not take the workflow down",
    PRELUDE + '''
class Hostile(Exception):
    def __str__(self):
        raise RuntimeError("cannot render")

def raiser():
    raise Hostile()

@workflow
async def main():
    caught = "none"
    try:
        await step("s", raiser)
    except TaskError as e:
        caught = type(e).__name__
    return {"caught": caught}
''',
    lambda j: None
    if j["success"] and j["result"]["caught"] == "TaskError"
    else f"success={j['success']} result={j.get('result')}",
)

case(
    "a non-finite attribute still reaches the checkpoint",
    PRELUDE + '''
class NotFinite(Exception):
    def __init__(self):
        super().__init__("nan")
        self.value = float("nan")

def raiser():
    raise NotFinite()

@workflow
async def main():
    try:
        await step("s", raiser)
    except TaskError as e:
        return {"extra": (e.result or {}).get("error", {}).get("extra")}
    return {"extra": None}
''',
    lambda j: None
    if j["success"] and "value" in (j["result"]["extra"] or {})
    else f"success={j['success']} result={j.get('result')} (NaN must not break the checkpoint)",
)

case(
    "an oversized stack is bounded but the step still completes",
    PRELUDE + '''
def deep(n):
    if n == 0:
        raise ValueError("deep " + "x" * 20000)
    return deep(n - 1)

def raiser():
    return deep(60)

@workflow
async def main():
    try:
        await step("s", raiser)
    except TaskError as e:
        stack = (e.result or {}).get("error", {}).get("stack") or ""
        return {"len": len(stack), "bounded": len(stack) <= 9000}
    return {"len": -1}
''',
    lambda j: None
    if j["success"] and j["result"]["bounded"]
    else f"success={j['success']} result={j.get('result')}",
)

case(
    "an oversized extra is dropped and flagged",
    PRELUDE + '''
class Huge(Exception):
    def __init__(self):
        super().__init__("huge")
        self.body = "y" * 200000

def raiser():
    raise Huge()

@workflow
async def main():
    try:
        await step("s", raiser)
    except TaskError as e:
        err = (e.result or {}).get("error", {})
        return {"hasExtra": "extra" in err, "omitted": err.get("extra_omitted", False)}
    return {}
''',
    lambda j: None
    if j["result"].get("omitted") is True and j["result"].get("hasExtra") is False
    else f"got {json.dumps(j.get('result'))}",
)

case(
    "an uncaught step failure fails the job and keeps the user's frame in the log",
    PRELUDE + '''
def deep_down():
    raise ValueError("uncaught boom")

@workflow
async def main():
    await step("s", deep_down)
    return "unreachable"
''',
    lambda j: None
    if (not j["success"] and "deep_down" in j["logs"])
    else f"success={j['success']} deep_down_in_log={'deep_down' in j['logs']}",
)

case(
    "a task failure caught by a broad except is reported, not swallowed",
    PRELUDE + '''
@task
async def boom():
    raise ValueError("nope")

@workflow
async def main():
    try:
        await boom()
    except Exception as e:
        return {"caught": type(e).__name__, "isRecord": bool(getattr(e, "result", None))}
    return {"caught": None}
''',
    lambda j: None
    if (j["result"] or {}).get("caught") == "TaskError"
    else f"got {j['result']} (a broad except must still see the failure, cf #10366)",
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
        j = run(c["script"], lang="python3")
        if j.get("timeout"):
            verdict, detail = "TIMEOUT", str(j.get("id", ""))
        elif "error" in j:
            verdict, detail = "ERROR", str(j["error"])[:140]
        else:
            try:
                problem = c["check"](j)
            except Exception as e:
                problem = f"check raised {type(e).__name__}: {e} | result={json.dumps(j.get('result'))[:200]}"
            verdict, detail = ("ok", "") if problem is None else ("SURPRISE", problem)
        if verdict != "ok":
            failures += 1
        print(f"{c['name']:<{width}}  {verdict:<9} {detail}", flush=True)
    print(f"\n{len(CASES) - failures}/{len(CASES)} behaved as expected")


if __name__ == "__main__":
    main()
