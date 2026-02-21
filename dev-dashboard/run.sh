#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Build frontend
cd frontend
bun run build
cd ..

cleanup() {
  kill $BE_PID $FE_PID 2>/dev/null || true
}
trap cleanup EXIT

# Backend (production)
cd backend
bun run start 2>&1 | sed 's/^/[BE] /' &
BE_PID=$!
cd ..

# Frontend (preview built assets)
cd frontend
bun run preview 2>&1 | sed 's/^/[FE] /' &
FE_PID=$!
cd ..

wait
