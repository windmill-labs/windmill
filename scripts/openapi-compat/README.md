# OpenAPI Compatibility Classifier

This utility compares two dereferenced OpenAPI JSON files and reports changes that can break generated clients or strict validators.

It was added for the release-safety question in issue 8719. The classifier is intentionally conservative for request-side changes and configurable for response-side required fields.

## Run

```bash
node scripts/openapi-compat/classify-openapi-changes.mjs before.json after.json
```

By default, adding required fields to response schemas is reported as a warning. If you want strict generated-client compatibility, treat those as breaking:

```bash
node scripts/openapi-compat/classify-openapi-changes.mjs before.json after.json --response-required=breaking
```

The command exits with code 1 when it finds breaking changes and prints a JSON report.

## What It Detects

- removed operations
- removed parameters
- optional parameters becoming required
- new required parameters
- request body becoming required
- new required request schema fields
- enum value removals
- removed component schemas
- new required response fields, warning by default or breaking in strict mode

The script expects JSON. Use `backend/windmill-api/openapi-deref.json` or another dereferenced OpenAPI JSON artifact when comparing releases.
