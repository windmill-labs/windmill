# Windmill Flow Building Guide

## CLI Commands

Create a folder ending with `__flow` and add a `flow.yaml` file with the flow definition.
For rawscript modules, use `!inline path/to/script.ts` for the content key. Inline script files should NOT include `.inline_script.` in their names (e.g. use `a.ts`, not `a.inline_script.ts`).
After writing, tell the user they can run:
- `wmill generate-metadata` - Generate lock files for the flow you modified
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

## OpenFlow Schema

The OpenFlow schema (openflow.openapi.yaml) is the source of truth for flow structure. Refer to OPENFLOW_SCHEMA for the complete type definitions.

## Reserved Module IDs

- `failure` - Reserved for failure handler module
- `preprocessor` - Reserved for preprocessor module
- `Input` - Reserved for flow input reference

## Hard Structural Rules

These are strict Windmill schema rules. Follow them exactly.

- `value.modules` is only for normal sequential steps
- `value.preprocessor_module` and `value.failure_module` are special top-level fields inside `value`, not entries in `value.modules`
- If a flow needs a preprocessor, create `value.preprocessor_module` with `id: preprocessor`
- If a flow needs a failure handler, create `value.failure_module` with `id: failure`
- Do NOT create regular modules inside `value.modules` named `preprocessor` or `failure`
- `preprocessor_module` and `failure_module` only support `script` or `rawscript`
- `preprocessor_module` runs before normal modules and cannot reference `results.*`
- `failure_module` can use the `error` object with `error.message`, `error.step_id`, `error.name`, and `error.stack`

Correct shape:

```yaml
value:
  preprocessor_module:
    id: preprocessor
    value:
      type: rawscript
      ...
  failure_module:
    id: failure
    value:
      type: rawscript
      ...
  modules:
    - id: process_event
      value:
        type: rawscript
        ...
```

Incorrect shape:

```yaml
value:
  modules:
    - id: preprocessor
      ...
    - id: process_event
      ...
    - id: failure
      ...
```

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
- `results.step_id` - Access output from a previous step only when that step result is in scope
- `results.step_id.property` - Access specific property from a previous step output only when that step result is in scope
- `flow_input.iter.value` - Current iteration value when inside a loop (`forloopflow` or `whileloopflow`)
- `flow_input.iter.index` - Current loop index when inside a loop (`forloopflow` or `whileloopflow`)

## Loop Structure Rules

- For `whileloopflow`, use module-level `stop_after_if` on the loop module itself when the loop should stop after an iteration result
- Do NOT put `stop_after_if` inside `value` of a `whileloopflow`
- `stop_after_all_iters_if` is for checks after the whole loop finishes, not the normal per-iteration break condition
- When a `whileloopflow` carries state forward between iterations, use `flow_input.iter.value` as the current loop value and provide an explicit first-iteration fallback when needed
- Use `flow_input.iter.index` only when the loop logic is truly based on the iteration index, not as a replacement for the current loop value
- If the user asks for a final scalar/object after a loop, add a normal step after the loop that extracts the final value from the loop result instead of returning the whole loop result array

Correct `whileloopflow` shape:

```yaml
- id: loop_until_done
  stop_after_if:
    expr: result.done === true
    skip_if_stopped: false
  value:
    type: whileloopflow
    skip_failures: false
    modules:
      - id: advance_state
        value:
          type: rawscript
          input_transforms:
            state:
              type: javascript
              expr: flow_input.iter && flow_input.iter.value !== undefined ? flow_input.iter.value : flow_input.initial_state
- id: return_final_state
  value:
    type: rawscript
    input_transforms:
      final_state:
        type: javascript
        expr: results.loop_until_done[results.loop_until_done.length - 1]
```

Incorrect `whileloopflow` patterns:

```yaml
- id: loop_until_done
  value:
    type: whileloopflow
    stop_after_if:
      expr: result.done === true
```

```yaml
input_transforms:
  state:
    type: javascript
    expr: flow_input.iter.index
```

```yaml
input_transforms:
  final_state:
    type: javascript
    expr: results.loop_until_done
```

## Approval / Suspend Structure

- `suspend` belongs on the flow module object itself, as a sibling of `id` and `value`
- Never put `suspend` inside `value`

Correct shape:

```yaml
- id: request_approval
  suspend:
    required_events: 1
    resume_form:
      schema:
        type: object
        properties:
          comment:
            type: string
        required: [comment]
  value:
    type: identity
```

Incorrect shape:

```yaml
- id: request_approval
  value:
    type: rawscript
    suspend:
      required_events: 1
```

## Branch Result Scope Rules

- Inside a branch, you may reference earlier outer steps and earlier steps in the same branch
- Outside a `branchone`, do NOT reference ids of steps that only exist inside its branches or default branch. Use `results.<branchone_module_id>` instead
- Outside a `branchall`, do NOT reference ids of steps inside its branches. Use `results.<branchall_module_id>` instead
- If downstream steps need a stable shape after a branch, make each branch return the same fields
- When needed, add a normalization step immediately after the branch and consume `results.<branch_module_id>` there

Correct after `branchone`:

```yaml
- id: route_order
  value:
    type: branchone
    ...
- id: send_confirmation
  value:
    input_transforms:
      routed:
        type: javascript
        expr: results.route_order
```

Incorrect after `branchone`:

```yaml
expr: results.create_shipment
expr: results.create_backorder
```

Correct after `branchall`:

```yaml
- id: enrich_parallel
  value:
    type: branchall
    parallel: true
    ...
- id: combine_data
  value:
    input_transforms:
      enrichments:
        type: javascript
        expr: results.enrich_parallel
```

## Input Transforms

Every rawscript module needs `input_transforms` to map function parameters to values:

Static transform (fixed value):
{"param_name": {"type": "static", "value": "fixed_string"}}

JavaScript transform (dynamic expression):
{"param_name": {"type": "javascript", "expr": "results.previous_step.data"}}

## Resource References

- For flow inputs: Use type `"object"` with format `"resource-{type}"` (e.g., `"resource-postgresql"`)
- For step inputs: Use static value `"$res:path/to/resource"`

## Final Structural Self-Check

Before finalizing a flow, verify:

- any preprocessor is in `value.preprocessor_module`
- any failure handler is in `value.failure_module`
- any approval step has module-level `suspend`
- no downstream step references inner branch step ids from outside the branch

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
