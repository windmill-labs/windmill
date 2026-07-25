# OSOP Workflow Example — Windmill API Data Sync

This directory contains a portable workflow definition for an **API data synchronization** pattern, written in [OSOP](https://github.com/osop-org/osop-spec) format.

## What is OSOP?

**OSOP** (Open Standard for Orchestration Protocols) is a YAML-based workflow standard that describes multi-step processes — including API integrations, data pipelines, and automation workflows — in a portable, tool-agnostic format. Think of it as "OpenAPI for workflows."

- Any tool can read and render an `.osop` file
- Workflows become shareable, diffable, and version-controllable
- No vendor lock-in: the same workflow runs across different orchestration engines

## Files

| File | Description |
|------|-------------|
| `api-data-sync.osop` | Webhook-triggered data sync: receive event, validate, enrich via external API (with retry), transform, store to database, and notify via Slack |

## How to use

You can read the `.osop` file as plain YAML. To validate or visualize it:

```bash
# Validate the workflow
pip install osop
osop validate api-data-sync.osop

# Generate a visual report
npx osop-report api-data-sync.osop -o report.html
```

## Learn more

- [OSOP Spec](https://github.com/osop-org/osop-spec) — Full specification
- [OSOP Examples](https://github.com/osop-org/osop-examples) — 30+ workflow templates
- [Windmill Documentation](https://www.windmill.dev/docs) — Windmill platform docs
