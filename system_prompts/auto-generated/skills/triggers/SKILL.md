---
name: triggers
description: MUST use when configuring triggers.
---

# Windmill Triggers

Triggers allow external events to invoke your scripts and flows.

## Supported Trigger Types

- HTTP routes - Custom API endpoints
- WebSocket - Real-time bidirectional communication
- Kafka - Apache Kafka consumer
- NATS - NATS messaging
- PostgreSQL CDC - Change Data Capture
- MQTT - IoT messaging protocol
- SQS - Amazon SQS queue
- GCP Pub/Sub - Google Cloud messaging

## File Naming

Trigger configuration files use the pattern: `{path}.{trigger_type}_trigger.json`

Examples:
- `u/user/webhook.http_trigger.json`
- `f/data/kafka_consumer.kafka_trigger.json`
- `f/sync/postgres_cdc.postgres_trigger.json`

## HTTP Trigger

```json
{
  "path": "u/user/webhook",
  "route_path": "/api/webhook",
  "script_path": "u/user/handler_script",
  "is_flow": false,
  "http_method": "POST",
  "authentication_method": "none",
  "is_async": false,
  "wrap_body": true,
  "raw_string": false
}
```

**Fields:**
- `route_path` - The HTTP path (e.g., `/api/webhook`)
- `http_method` - GET, POST, PUT, DELETE, or PATCH
- `authentication_method` - none, windmill, api_key, basic_http, custom_script, or signature
- `is_async` - If true, returns immediately and runs in background
- `wrap_body` - Wrap request body in a parameter
- `raw_string` - Return raw string response

## WebSocket Trigger

```json
{
  "path": "u/user/ws_listener",
  "url": "wss://example.com/socket",
  "script_path": "u/user/ws_handler",
  "is_flow": false,
  "filters": [{"key": "type", "value": "update"}],
  "initial_messages": [{"type": "static", "value": {"action": "subscribe"}}],
  "can_return_message": true
}
```

**Fields:**
- `url` - WebSocket URL to connect to
- `filters` - Filter incoming messages by key-value pairs
- `initial_messages` - Messages to send on connection
- `can_return_message` - Script can send messages back

## Kafka Trigger

```json
{
  "path": "u/user/kafka_consumer",
  "kafka_resource_path": "f/resources/kafka",
  "group_id": "windmill-consumer-group",
  "topics": ["events", "notifications"],
  "script_path": "u/user/kafka_handler",
  "is_flow": false
}
```

**Fields:**
- `kafka_resource_path` - Path to Kafka resource (connection config)
- `group_id` - Kafka consumer group ID
- `topics` - List of topics to consume

## NATS Trigger

```json
{
  "path": "u/user/nats_subscriber",
  "nats_resource_path": "f/resources/nats",
  "subjects": ["events.>", "updates.*"],
  "use_jetstream": false,
  "script_path": "u/user/nats_handler",
  "is_flow": false
}
```

**Fields:**
- `nats_resource_path` - Path to NATS resource
- `subjects` - NATS subjects to subscribe (supports wildcards)
- `use_jetstream` - Enable JetStream for persistent messaging
- `stream_name`, `consumer_name` - Required when using JetStream

## PostgreSQL CDC Trigger

```json
{
  "path": "u/user/postgres_cdc",
  "postgres_resource_path": "f/resources/postgres",
  "publication_name": "windmill_publication",
  "replication_slot_name": "windmill_slot",
  "script_path": "u/user/cdc_handler",
  "is_flow": false
}
```

**Fields:**
- `postgres_resource_path` - Path to PostgreSQL resource
- `publication_name` - PostgreSQL publication name
- `replication_slot_name` - Replication slot name

## MQTT Trigger

```json
{
  "path": "u/user/mqtt_subscriber",
  "mqtt_resource_path": "f/resources/mqtt",
  "subscribe_topics": [{"topic": "sensors/+/temperature", "qos": 1}],
  "client_version": "v5",
  "client_id": "windmill-mqtt-client",
  "script_path": "u/user/mqtt_handler",
  "is_flow": false
}
```

**Fields:**
- `mqtt_resource_path` - Path to MQTT resource
- `subscribe_topics` - List of topics with QoS settings
- `client_version` - v3 or v5
- `client_id` - Optional client identifier

## SQS Trigger

```json
{
  "path": "u/user/sqs_consumer",
  "queue_url": "https://sqs.us-east-1.amazonaws.com/123456789/my-queue",
  "aws_resource_path": "f/resources/aws",
  "aws_auth_resource_type": "credentials",
  "message_attributes": ["attribute1", "attribute2"],
  "script_path": "u/user/sqs_handler",
  "is_flow": false
}
```

**Fields:**
- `queue_url` - Full SQS queue URL
- `aws_resource_path` - Path to AWS credentials resource
- `aws_auth_resource_type` - credentials or oidc
- `message_attributes` - SQS message attributes to include

## GCP Pub/Sub Trigger

```json
{
  "path": "u/user/gcp_subscriber",
  "gcp_resource_path": "f/resources/gcp",
  "topic_id": "my-topic",
  "subscription_id": "my-subscription",
  "subscription_mode": "create_update",
  "delivery_type": "pull",
  "script_path": "u/user/gcp_handler",
  "is_flow": false
}
```

**Fields:**
- `gcp_resource_path` - Path to GCP credentials resource
- `topic_id` - Pub/Sub topic ID
- `subscription_id` - Subscription ID
- `subscription_mode` - existing or create_update
- `delivery_type` - push or pull

## Error Handling

All triggers support error handlers:

```json
{
  "error_handler_path": "u/user/error_handler",
  "error_handler_args": {"notify": true},
  "retry": {
    "max_retries": 3,
    "max_wait": 300,
    "multiplier": 2
  }
}
```

## CLI Commands

```bash
# List triggers
wmill trigger list

# Push trigger configuration
wmill sync push
```
