---
name: triggers
description: MUST use when configuring triggers.
---

# Windmill Triggers

Triggers allow external events to invoke your scripts and flows.

## File Naming

Trigger configuration files use the pattern: `{path}.{trigger_type}_trigger.yaml`

Examples:
- `u/user/webhook.http_trigger.yaml`
- `f/data/kafka_consumer.kafka_trigger.yaml`
- `f/sync/postgres_cdc.postgres_trigger.yaml`

## CLI Commands

```bash
# Push trigger configuration
wmill sync push

# Pull triggers from Windmill
wmill sync pull
```
