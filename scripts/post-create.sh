#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/worktree-common.sh"

backend_port="${BACKEND_PORT:-}"
frontend_port="${FRONTEND_PORT:-}"

if [[ -z "$backend_port" || -z "$frontend_port" ]]; then
  echo "Missing BACKEND_PORT or FRONTEND_PORT in hook environment" >&2
  exit 1
fi

wm_write_env_local "$(pwd)" "$backend_port" "$frontend_port"
wm_shared_post_create "$(pwd)"
