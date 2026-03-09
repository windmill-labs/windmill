#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/worktree-common.sh"

backend_port="${BACKEND_PORT:-}"
frontend_port="${FRONTEND_PORT:-}"

if [[ -z "$backend_port" || -z "$frontend_port" ]]; then
  echo "Missing BACKEND_PORT or FRONTEND_PORT in hook environment" >&2
  exit 1
fi

cat > .env.local <<EOF
BACKEND_PORT=$backend_port
FRONTEND_PORT=$frontend_port
REMOTE=http://localhost:$backend_port
EOF

if [[ -n "${CARGO_FEATURES:-}" ]]; then
  echo "CARGO_FEATURES=$CARGO_FEATURES" >> .env.local
fi

echo "Created .env.local with ports: backend=$backend_port, frontend=$frontend_port"
wm_shared_post_create "$(pwd)"
