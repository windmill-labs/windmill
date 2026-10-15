#!/usr/bin/env python3
"""cancel.py <tag> <worker_pid> : start a long DuckDB job, cancel it after 3s, report how long the
job takes to be marked canceled and how much CPU the worker's process tree keeps burning after."""
import json, os, sys, time, subprocess
sys.path.insert(0, os.path.dirname(__file__))
from run import call

SQL = "SELECT count(*) FROM range(40000000000) t(i) WHERE (i * 7919) % 104729 = 3;"


def tree_cpu_ticks(pid):
    pids = subprocess.run(["pgrep", "-g", str(os.getpgid(pid))], capture_output=True, text=True).stdout.split()
    # the worker's own process plus every descendant (nsjail + runner live in their own group)
    out = subprocess.run(["ps", "-e", "-o", "pid=,ppid="], capture_output=True, text=True).stdout
    children = {}
    for line in out.splitlines():
        p, pp = line.split()
        children.setdefault(int(pp), []).append(int(p))
    todo, seen = [pid], set()
    while todo:
        p = todo.pop()
        if p in seen:
            continue
        seen.add(p)
        todo.extend(children.get(p, []))
    total = 0
    for p in seen:
        try:
            f = open(f"/proc/{p}/stat").read().rsplit(")", 1)[1].split()
            total += int(f[11]) + int(f[12])
        except OSError:
            pass
    return total


def cpu_rate(pid, secs=2.0):
    a = tree_cpu_ticks(pid)
    time.sleep(secs)
    return (tree_cpu_ticks(pid) - a) / os.sysconf("SC_CLK_TCK") / secs


tag, wpid = sys.argv[1], int(sys.argv[2])
st, jid = call("/jobs/run/preview", {"content": SQL, "language": "duckdb", "args": {}, "tag": tag})
jid = jid.strip().strip('"')
# wait until running
while True:
    st, j = call(f"/jobs_u/get/{jid}")
    j = json.loads(j)
    if j.get("running"):
        break
    time.sleep(0.05)
time.sleep(3)
busy = cpu_rate(wpid, 1.0)
t0 = time.perf_counter()
st, r = call(f"/jobs_u/queue/cancel/{jid}", {"reason": "bench"})
while True:
    st, j = call(f"/jobs_u/get/{jid}")
    j = json.loads(j)
    if j.get("type") == "CompletedJob":
        break
    time.sleep(0.02)
marked = time.perf_counter() - t0
after = [round(cpu_rate(wpid, 2.0), 2) for _ in range(3)]
print(json.dumps({"tag": tag, "cpu_cores_before_cancel": round(busy, 2), "cancel_to_completed_s": round(marked, 3),
                  "cpu_cores_after_cancel_0_2_4_6s": after}))
