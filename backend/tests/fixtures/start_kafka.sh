#!/usr/bin/env bash
# Starts a Kafka broker for trigger_e2e::test_kafka_e2e (Enterprise)
#
# Usage:
#   ./tests/fixtures/start_kafka.sh        # start
#   ./tests/fixtures/start_kafka.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-kafka"
PORT=9092

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:9092" \
    -e KAFKA_NODE_ID=1 \
    -e KAFKA_PROCESS_ROLES=broker,controller \
    -e KAFKA_LISTENERS="PLAINTEXT://0.0.0.0:${PORT},CONTROLLER://0.0.0.0:9093" \
    -e KAFKA_LISTENER_SECURITY_PROTOCOL_MAP=CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT \
    -e KAFKA_CONTROLLER_QUORUM_VOTERS=1@localhost:9093 \
    -e KAFKA_CONTROLLER_LISTENER_NAMES=CONTROLLER \
    -e KAFKA_ADVERTISED_LISTENERS="PLAINTEXT://localhost:${PORT}" \
    -e KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1 \
    -e KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS=0 \
    apache/kafka:latest

echo "Waiting for Kafka to become ready..."
for i in $(seq 1 60); do
    if docker exec "$NAME" /opt/kafka/bin/kafka-topics.sh --list --bootstrap-server "localhost:${PORT}" &>/dev/null; then
        break
    fi
    sleep 1
done

docker exec "$NAME" /opt/kafka/bin/kafka-topics.sh --create \
    --topic windmill-e2e-test \
    --bootstrap-server "localhost:${PORT}" \
    --partitions 1 --replication-factor 1 2>/dev/null || true

echo "Kafka broker listening on localhost:${PORT} with topic 'windmill-e2e-test'"
echo ""
echo "Run the test:"
echo "  cargo test --test trigger_e2e test_kafka_e2e --features kafka,enterprise,private -- --ignored --nocapture"
