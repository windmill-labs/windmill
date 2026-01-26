---
name: write-script-bash
description: Write Bash scripts with positional arguments and JSON output.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# Windmill Script Writing Guide

## General Principles

- Scripts must export a main function (do not call it)
- Libraries are installed automatically - do not show installation instructions
- Credentials and configuration are stored in resources and passed as parameters
- The windmill client (`wmill`) provides APIs for interacting with the platform

## Function Naming

- Main function: `main` (or `preprocessor` for preprocessor scripts)
- Must be async for TypeScript variants

## Return Values

- Scripts can return any JSON-serializable value
- Return values become available to subsequent flow steps via `results.step_id`

## Preprocessor Scripts

Preprocessor scripts process raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, Postgres, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.

The returned object determines the parameter values passed to the flow.
e.g., `{ b: 1, a: 2 }` calls the flow with `a = 2` and `b = 1`, assuming the flow has two inputs called `a` and `b`.

The preprocessor receives a single parameter called `event`.


# Bash

## Structure

Do not include `#!/bin/bash`. Arguments are obtained as positional parameters:

```bash
# Get arguments
var1="$1"
var2="$2"

echo "Processing $var1 and $var2"

# Return JSON by echoing to stdout
echo "{\"result\": \"$var1\", \"count\": $var2}"
```

**Important:**
- Do not include shebang (`#!/bin/bash`)
- Arguments are always strings
- Access with `$1`, `$2`, etc.

## Output

The script output is captured as the result. For structured data, output valid JSON:

```bash
name="$1"
count="$2"

# Output JSON result
cat << EOF
{
  "name": "$name",
  "count": $count,
  "timestamp": "$(date -Iseconds)"
}
EOF
```

## Environment Variables

Environment variables set in Windmill are available:

```bash
# Access environment variable
echo "Workspace: $WM_WORKSPACE"
echo "Job ID: $WM_JOB_ID"
```
