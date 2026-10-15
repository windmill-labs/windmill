#!/bin/bash
# Decomposes the per-job cost of the out-of-process path outside any worker:
#   nsjail running /bin/true                 = jail setup alone
#   runner, SELECT 1, unjailed               = process + engine + httpfs/azure setup
#   runner, SELECT 1, in nsjail              = the above + jail
#   runner, SELECT 1 + fence preloads        = what the fence adds in a fresh process
# usage: micro.sh <bin_dir> [reps]

S=$(cd "$(dirname "$0")" && pwd); R=$1; N=${2:-30}
J=$(mktemp -d); cd "$J"
PROTO=$S/../../windmill-worker/nsjail/run.duckdb.config.proto python3 - "$R" "$J" <<'EOF'
import os, sys
R, J = sys.argv[1], sys.argv[2]
s = open(os.environ['PROTO']).read()
tmp = 'mount {\n    dst: "/tmp"\n    fstype: "tmpfs"\n    rw: true\n    options: "size=500000000"\n}'
import os
s = (s.replace('{JOB_DIR}', J).replace('{RUNNER_PATH}', R + '/windmill_duckdb_runner')
     .replace('{EXTENSION_DIR}', os.path.expanduser('~/.duckdb/extensions')).replace('{BIGQUERY_CREDENTIALS_MOUNT}', '')
     .replace('{CLONE_NEWUSER}', 'true').replace('#{DEV}', '').replace('{TMP_MOUNT_BLOCK}', tmp))
open('cfg.proto', 'w').write(s)
s2 = s.replace('dst: "/tmp/windmill_duckdb_runner"', 'dst: "/tmp/windmill_duckdb_runner"\n    is_bind: true\n}\nmount {\n    src: "/bin/true"\n    dst: "/tmp/true"')
open('cfg_true.proto', 'w').write(s2)
EOF
req() { # $1 = blocks json array, $2 = lock
  echo '{"mode":"run","blocks":'"$1"',"job_args":[],"token":"a.b","base_internal_url":"http://localhost:8000","w_id":"admins","memory_limit":"","temp_directory":"'$J'","max_result_size":0,"collect_last_only":true,"collect_first_row_only":false,"result_path":"'$J'/out","lock_temp_directory":'"$2"'}'
}
PLAIN='["SELECT 1 AS x;"]'
FENCED='["INSTALL parquet;","LOAD parquet;","INSTALL json;","LOAD json;","SET lock_temp_directory=true;","SET disabled_filesystems='"'"'LocalFileSystem'"'"';","SET allow_community_extensions=false;","SELECT 1 AS x;"]'
measure() { # name, cmd...
  local name=$1; shift
  local ts=()
  for i in $(seq 1 $N); do
    local a=$(date +%s%N); "$@" > /dev/null 2>&1; local b=$(date +%s%N); ts+=($(( (b - a) / 1000 )))
  done
  printf '%s\n' "${ts[@]}" | sort -n | awk -v n="$name" '{a[NR]=$1} END {printf "%-34s p50 %6.1f ms  p95 %6.1f ms\n", n, a[int(NR*0.5)+1]/1000, a[int(NR*0.95)+1]/1000}'
}
export HOME
run_plain() { req "$PLAIN" false | env -i HOME=$HOME PATH=/usr/bin:/bin $R/windmill_duckdb_runner; }
run_fenced() { req "$FENCED" false | env -i HOME=$HOME PATH=/usr/bin:/bin $R/windmill_duckdb_runner; }
jail_plain() { req "$PLAIN" false | env -i PATH=/usr/bin:/bin /usr/bin/nsjail --config cfg.proto -- /tmp/windmill_duckdb_runner; }
jail_fenced() { req "$FENCED" false | env -i PATH=/usr/bin:/bin /usr/bin/nsjail --config cfg.proto -- /tmp/windmill_duckdb_runner; }
jail_true() { env -i PATH=/usr/bin:/bin /usr/bin/nsjail --config cfg_true.proto -- /tmp/true; }
measure "nsjail + /bin/true" jail_true
measure "runner SELECT 1 (unjailed)" run_plain
measure "runner SELECT 1 (nsjail)" jail_plain
measure "runner fenced SELECT 1 (unjailed)" run_fenced
measure "runner fenced SELECT 1 (nsjail)" jail_fenced
