#!/bin/bash
# usage: worker.sh <bin_dir> <tag> <port> [extra env...]
BIN=$1; TAG=$2; P=$3; shift 3
exec env -i HOME=$HOME PATH=/usr/local/bin:/usr/bin:/bin USER=$USER \
  DATABASE_URL="$DATABASE_URL" \
  MODE=worker PORT=$P WORKER_TAGS=$TAG WORKER_GROUP=$TAG BASE_INTERNAL_URL=${WM_BASE:-http://localhost:8000} \
  LD_LIBRARY_PATH=$BIN DUCKDB_RUNNER_PATH=$BIN/windmill_duckdb_runner RUST_LOG=info \
  DISABLE_NSJAIL=false MAX_SQL_RESULT_SIZE=256MB "$@" $BIN/windmill
