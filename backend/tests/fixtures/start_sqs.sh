#!/usr/bin/env bash
# Starts LocalStack for SQS trigger_e2e::test_sqs_e2e (Enterprise)
#
# Usage:
#   ./tests/fixtures/start_sqs.sh        # start
#   ./tests/fixtures/start_sqs.sh stop   # stop & remove
set -euo pipefail

NAME="windmill-test-localstack"
PORT=4566

if [[ "${1:-}" == "stop" ]]; then
    docker rm -f "$NAME" 2>/dev/null && echo "stopped $NAME" || echo "$NAME not running"
    exit 0
fi

if docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
    echo "$NAME is already running"
    exit 0
fi

docker rm -f "$NAME" 2>/dev/null || true

docker run -d --name "$NAME" -p "${PORT}:4566" \
    -e SERVICES=sqs \
    localstack/localstack

echo "Waiting for LocalStack to become ready..."
for i in $(seq 1 30); do
    if curl -sf "http://localhost:${PORT}/_localstack/health" &>/dev/null; then
        break
    fi
    sleep 1
done

# Create the test queue
aws --endpoint-url="http://localhost:${PORT}" \
    --region us-east-1 \
    --no-sign-request \
    sqs create-queue --queue-name windmill-e2e-test 2>/dev/null || true

echo "LocalStack SQS listening on localhost:${PORT} with queue 'windmill-e2e-test'"
echo ""
echo "Run the test:"
echo "  AWS_ENDPOINT_URL=http://localhost:${PORT} cargo test --test trigger_e2e test_sqs_e2e --features sqs_trigger,enterprise,private -- --ignored --nocapture"
