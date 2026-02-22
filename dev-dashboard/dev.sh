#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Load env vars (R2 credentials, etc.) if present
if [ -f .env ]; then
  set -a; source .env; set +a
fi

cleanup() {
  kill $BE_PID $FE_PID 2>/dev/null || true
}
trap cleanup EXIT

# Backend (bun --watch)
cd backend
bun run dev 2>&1 | sed 's/^/[BE] /' &
BE_PID=$!
cd ..

# Frontend (vite dev)
cd frontend
bun run dev 2>&1 | sed 's/^/[FE] /' &
FE_PID=$!
cd ..

wait
