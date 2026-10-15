#!/bin/bash
# start_workers.sh <bin_dir> [workers per variant] [variants...]
S=$(dirname "$0"); R=$1; N=${2:-1}; shift 2; VARIANTS=${*:-"inproc proc jail nofence"}
cd "$S"
L=${LOG_DIR:-/tmp/wm-duckdb-bench-logs}; mkdir -p "$L"
start() { local t=$1 port=$2; shift 2; for i in $(seq 1 $N); do nohup ./worker.sh $R $t $((port + 10*i)) "$@" > "$L/rel_${t}_$i.log" 2>&1 & done; }
n=0
for v in $VARIANTS; do
  case $v in
    inproc) start bench_inproc 8162 ;;
    proc) start bench_proc 8163 DUCKDB_JAILED_RUNNER=process ;;
    jail) start bench_jail 8161 DUCKDB_JAILED_RUNNER=true ;;
    nofence) start bench_nofence 8164 DUCKDB_JAILED_RUNNER=true DUCKDB_JAILED_RUNNER_FENCE=false ;;
  esac
  n=$((n + N))
done
until [ $(grep -l "ping update" "$L"/rel_*.log 2>/dev/null | wc -l) -ge $n ]; do sleep 1; done
echo "$n workers up"
