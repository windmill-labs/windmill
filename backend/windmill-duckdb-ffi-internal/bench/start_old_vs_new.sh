#!/bin/bash
# start_old_vs_new.sh <old_bin_dir> <new_bin_dir> [workers per variant]
# Old = a build of main (in-process FFI + fence; its dir also holds libwindmill_duckdb_ffi_internal.so).
# New = this branch (windmill_duckdb_runner next to windmill). One tag per (build, isolation level).
S=$(dirname "$0"); OLD=$1; NEW=$2; N=${3:-1}; cd "$S"
L=${LOG_DIR:-/tmp/wm-duckdb-bench-logs}; mkdir -p "$L"
start() { local bin=$1 t=$2 port=$3; shift 3; for i in $(seq 1 $N); do nohup ./worker.sh $bin $t $((port + 10*i)) "$@" > "$L/rel_${t}_$i.log" 2>&1 & done; }
start $OLD old_nsjail 8500
start $OLD old_unshare 8501 DISABLE_NSJAIL=true ENABLE_UNSHARE_PID=true
start $OLD old_none 8502 DISABLE_NSJAIL=true
start $NEW new_nsjail 8503
start $NEW new_unshare 8504 DISABLE_NSJAIL=true ENABLE_UNSHARE_PID=true
start $NEW new_none 8505 DISABLE_NSJAIL=true
until [ $(grep -l "ping update" "$L"/rel_{old,new}_*.log 2>/dev/null | wc -l) -ge $((6 * N)) ]; do sleep 1; done
echo "$((6*N)) workers up"
