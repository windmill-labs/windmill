#!/usr/bin/env python3
"""memwatch.py <tag> <worker_pid> <sql-file> : run one job while sampling RSS of the worker
process and of its descendants (nsjail + runner) every 20ms. Reports peaks and the worker's RSS
before and 3s after."""
import json, os, subprocess, sys, threading, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run import run_wait


def rss_kb(pid):
    try:
        for line in open(f"/proc/{pid}/status"):
            if line.startswith("VmRSS:"):
                return int(line.split()[1])
    except OSError:
        pass
    return 0


def descendants(pid):
    out = subprocess.run(["ps", "-e", "-o", "pid=,ppid="], capture_output=True, text=True).stdout
    ch = {}
    for l in out.splitlines():
        p, pp = map(int, l.split())
        ch.setdefault(pp, []).append(p)
    todo, seen = list(ch.get(pid, [])), []
    while todo:
        p = todo.pop()
        seen.append(p)
        todo.extend(ch.get(p, []))
    return seen


tag, wpid, sqlf = sys.argv[1], int(sys.argv[2]), sys.argv[3]
sql = open(sqlf).read() if os.path.exists(sqlf) else sqlf
before = rss_kb(wpid)
peak_w, peak_c, stop = [before], [0], [False]


def sample():
    while not stop[0]:
        peak_w[0] = max(peak_w[0], rss_kb(wpid))
        peak_c[0] = max(peak_c[0], sum(rss_kb(p) for p in descendants(wpid)))
        time.sleep(0.02)


th = threading.Thread(target=sample)
th.start()
st, out, dt = run_wait(tag, sql)
stop[0] = True
th.join()
time.sleep(3)
print(json.dumps({"tag": tag, "status": st, "wall_ms": round(dt * 1000), "out": out[:200],
                  "worker_rss_before_mb": before // 1024, "worker_rss_peak_mb": peak_w[0] // 1024,
                  "children_rss_peak_mb": peak_c[0] // 1024, "worker_rss_after_3s_mb": rss_kb(wpid) // 1024}))
