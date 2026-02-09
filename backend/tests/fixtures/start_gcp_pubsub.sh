#!/usr/bin/env bash
# Starts the GCP Pub/Sub emulator for trigger_e2e::test_gcp_e2e (Enterprise)
#
# Usage:
#   ./tests/fixtures/start_gcp_pubsub.sh        # start
#   ./tests/fixtures/start_gcp_pubsub.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-pubsub"
PORT=8085

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:8085" \
    gcr.io/google.com/cloudsdktool/google-cloud-cli:emulators \
    gcloud beta emulators pubsub start --host-port="0.0.0.0:${PORT}"

echo "Waiting for Pub/Sub emulator to become ready..."
for i in $(seq 1 30); do
    if curl -sf "http://localhost:${PORT}" &>/dev/null; then
        break
    fi
    sleep 1
done

# Create the test topic and subscription
curl -sX PUT "http://localhost:${PORT}/v1/projects/local-project/topics/windmill-e2e-test" >/dev/null
curl -sX PUT "http://localhost:${PORT}/v1/projects/local-project/subscriptions/windmill-e2e-sub" \
    -H "Content-Type: application/json" \
    -d '{"topic": "projects/local-project/topics/windmill-e2e-test"}' >/dev/null

echo "GCP Pub/Sub emulator listening on localhost:${PORT}"
echo "  topic:        windmill-e2e-test"
echo "  subscription: windmill-e2e-sub"
echo ""
echo "Run the test:"
echo "  PUBSUB_EMULATOR_HOST=localhost:${PORT} cargo test --test trigger_e2e test_gcp_e2e --features gcp_trigger,enterprise,private -- --ignored --nocapture"
