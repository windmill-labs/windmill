#!/usr/bin/env bash
# Starts (or stops) all external services needed for the trigger e2e tests.
#
# Usage:
#   ./tests/fixtures/start_all_triggers.sh        # start everything
#   ./tests/fixtures/start_all_triggers.sh stop   # stop everything
#   ./tests/fixtures/start_all_triggers.sh oss    # start only OSS services
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
ACTION="${1:-start}"

SCRIPTS_OSS=(
    "$DIR/start_mqtt.sh"
    "$DIR/start_websocket.sh"
    "$DIR/start_postgres_replication.sh"
)

SCRIPTS_EE=(
    "$DIR/start_kafka.sh"
    "$DIR/start_nats.sh"
    "$DIR/start_sqs.sh"
    "$DIR/start_gcp_pubsub.sh"
)

if [[ "$ACTION" == "stop" ]]; then
    for s in "${SCRIPTS_OSS[@]}" "${SCRIPTS_EE[@]}"; do
        echo "--- $(basename "$s" .sh) stop ---"
        bash "$s" stop
    done
    exit 0
fi

if [[ "$ACTION" == "oss" ]]; then
    SCRIPTS=("${SCRIPTS_OSS[@]}")
else
    SCRIPTS=("${SCRIPTS_OSS[@]}" "${SCRIPTS_EE[@]}")
fi

for s in "${SCRIPTS[@]}"; do
    echo "--- $(basename "$s" .sh) ---"
    bash "$s"
    echo ""
done

echo "============================================"
echo "All services ready. Run the e2e tests with:"
echo ""

if [[ "$ACTION" == "oss" ]]; then
    echo "  cargo test --test trigger_e2e --features mqtt_trigger,websocket,postgres_trigger -- --ignored --nocapture"
else
    echo "  # OSS triggers"
    echo "  cargo test --test trigger_e2e --features mqtt_trigger,websocket,postgres_trigger -- --ignored --nocapture"
    echo ""
    echo "  # Enterprise triggers"
    echo "  AWS_ENDPOINT_URL=http://localhost:4566 PUBSUB_EMULATOR_HOST=localhost:8085 cargo test --test trigger_e2e --features kafka,nats,sqs_trigger,gcp_trigger,enterprise,private -- --ignored --nocapture"
fi
