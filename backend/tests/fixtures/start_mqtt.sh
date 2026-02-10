#!/usr/bin/env bash
# Starts a Mosquitto MQTT broker for trigger_e2e::test_mqtt_e2e
#
# Usage:
#   ./tests/fixtures/start_mqtt.sh        # start
#   ./tests/fixtures/start_mqtt.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-mqtt"
PORT=1883

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:1883" \
    eclipse-mosquitto:latest \
    mosquitto -c /mosquitto-no-auth.conf

echo "MQTT broker listening on localhost:${PORT}"
echo ""
echo "Run the test:"
echo "  cargo test --test trigger_e2e test_mqtt_e2e --features mqtt_trigger -- --ignored --nocapture"
