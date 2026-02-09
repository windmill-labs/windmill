#!/usr/bin/env bash
# Starts a NATS server for trigger_e2e::test_nats_e2e (Enterprise)
#
# Usage:
#   ./tests/fixtures/start_nats.sh        # start
#   ./tests/fixtures/start_nats.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-nats"
PORT=4222

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:4222" nats:latest

echo "NATS server listening on localhost:${PORT}"
echo ""
echo "Run the test:"
echo "  cargo test --test trigger_e2e test_nats_e2e --features nats,enterprise,private -- --ignored --nocapture"
