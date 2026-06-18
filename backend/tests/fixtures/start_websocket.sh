#!/usr/bin/env bash
# Starts a WebSocket echo server for trigger_e2e::test_websocket_e2e
#
# Usage:
#   ./tests/fixtures/start_websocket.sh        # start
#   ./tests/fixtures/start_websocket.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-ws-echo"
PORT=8765

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:8080" \
    -e PORT=8080 \
    jmalloc/echo-server

echo "WebSocket echo server listening on localhost:${PORT}"
echo ""
echo "Run the test:"
echo "  cargo test --test trigger_e2e test_websocket_e2e --features websocket -- --ignored --nocapture"
