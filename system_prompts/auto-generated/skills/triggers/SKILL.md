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
- `f/inbound/orders.email_trigger.yaml`

## Email Triggers

An email trigger routes incoming emails to a script or flow. Each trigger reserves a local-part: emails sent to `<local_part>@<windmill_email_domain>` are delivered to the configured runnable. Set `workspaced_local_part: true` to namespace it per workspace (the actual recipient becomes `<workspace_id>-<local_part>@…`); on Windmill Cloud this is required.

Senders may append URL-style extras to the local-part with `+`: `mytrigger+foo=bar+baz=qux@…`. They flow through to the script as `email_extra_args` (see below).

### Payload

The runnable receives:

- `parsed_email` — `{ headers, text_body, html_body, attachments[] }`. Each `attachment` has `{ headers, body }`.
- `raw_email` — the raw RFC 822 message as a string, **or** an S3 object (`{ s3: "windmill_emails/<job_id>/raw.eml" }`) if the message exceeds 1 MiB.
- `email_extra_args` (optional, only when sender appended `+key=value` extras) — a flat object of the parsed extras.

With a preprocessor, all of the above are nested under `event` along with `event.kind = "email"` and `event.trigger_path` (the trigger's path). Without a preprocessor, `trigger_path` is **not** exposed — add a preprocessor if you need it.

### Attachments are S3 objects

Binary attachments are uploaded to the workspace S3 bucket and surface in `parsed_email.attachments[i].body` as:

```json
{ "s3": "windmill_emails/<job_id>/attachments/<filename>" }
```

To read the bytes inside a script, use the wmill SDK:

```ts
// TypeScript
import * as wmill from "windmill-client"
const file = await wmill.loadS3File(parsed_email.attachments[0].body)
```

```python
# Python
import wmill
data = wmill.load_s3_file(parsed_email["attachments"][0]["body"])
```

If the workspace has no S3 resource configured (Workspace Settings → Object storage), `body` falls back to the string `"configure s3 in the workspace settings to handle attachments"`. The same applies to large `raw_email` bodies. Email attachment storage requires the server to be built with the `parquet` feature.

Text/HTML/inline parts are placed inline in `body` as strings.

## CLI Commands

After writing, tell the user they can run these commands (do NOT run them yourself):

```bash
# Push trigger configuration
wmill sync push

# Pull triggers from Windmill
wmill sync pull
```
