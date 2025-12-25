# Windmill Flow Building Guide

The OpenFlow schema (openflow.openapi.yaml) is the source of truth for flow structure. Refer to OPENFLOW_SCHEMA for the complete type definitions.

## Reserved Module IDs

- `failure` - Reserved for failure handler module
- `preprocessor` - Reserved for preprocessor module
- `Input` - Reserved for flow input reference

## Module ID Rules

- Must be unique across the entire flow
- Use underscores, not spaces (e.g., `fetch_data` not `fetch data`)
- Use descriptive names that reflect the step's purpose

## Common Mistakes to Avoid

- Missing `input_transforms` - Rawscript parameters won't receive values without them
- Referencing future steps - `results.step_id` only works for steps that execute before the current one
- Duplicate module IDs - Each module ID must be unique in the flow

## Data Flow Between Steps

- `flow_input.property` - Access flow input parameters
- `results.step_id` - Access output from a previous step
- `results.step_id.property` - Access specific property from previous step output
- `flow_input.iter.value` - Current item when inside a for-loop
- `flow_input.iter.index` - Current index when inside a for-loop

## Input Transforms

Every rawscript module needs `input_transforms` to map function parameters to values:

Static transform (fixed value):
{"param_name": {"type": "static", "value": "fixed_string"}}

JavaScript transform (dynamic expression):
{"param_name": {"type": "javascript", "expr": "results.previous_step.data"}}

## Resource References

- For flow inputs: Use type `"object"` with format `"resource-{type}"` (e.g., `"resource-postgresql"`)
- For step inputs: Use static value `"$res:path/to/resource"`

## Failure Handler

Executes when any step fails. Has access to error details:

- `error.message` - Error message
- `error.step_id` - ID of failed step
- `error.name` - Error name
- `error.stack` - Stack trace

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

To accept an S3 object as flow input:

```json
{
  "type": "object",
  "properties": {
    "file": {
      "type": "object",
      "format": "resource-s3_object",
      "description": "File to process"
    }
  }
}
```

## Using Resources in Flows

On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.

### As Flow Input

In the flow schema, set the property type to `"object"` with format `"resource-{type}"`:

```json
{
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "format": "resource-postgresql",
      "description": "Database connection"
    }
  }
}
```

### As Step Input (Static Reference)

Reference a specific resource using `$res:` prefix:

```json
{
  "database": {
    "type": "static",
    "value": "$res:f/folder/my_database"
  }
}
```
