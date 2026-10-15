#!/bin/bash
# First DuckDB job on a freshly started worker vs its second one, per variant, 3 restarts.
# usage: firstjob.sh <bin_dir>
S=$(cd "$(dirname "$0")" && pwd); R=$1; cd "$S"
kill_bench() {
  for p in $(pgrep -f "wm-duckdb-jail/rel/windmill$"); do
    tr '\0' '\n' < /proc/$p/environ | grep -q "WORKER_TAGS=bench_" && kill $p
  done
  sleep 5
}
kill_bench; sleep 3
for r in 1 2 3; do
  rm -f "${LOG_DIR:-/tmp/wm-duckdb-bench-logs}"/rel_bench_*
  bash start_workers.sh $R 1 inproc jail nofence > /dev/null
  python3 - <<'EOF'
import sys; sys.path.insert(0, '.')
from bench import exec_ms, db_now
from run import run_wait
for v in ["bench_inproc", "bench_jail", "bench_nofence"]:
    out = []
    for k in range(2):
        since = db_now(); st, o, dt = run_wait(v, "SELECT 1 AS x;"); out.append(exec_ms(v, since))
    print(v, "first", out[0], "second", out[1], flush=True)
EOF
  kill_bench; sleep 3
done
