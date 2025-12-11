# Windmill Flow Building Guide

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

### Static Transform
Fixed value passed directly:
```json
{
  "param_name": {
    "type": "static",
    "value": "fixed_string"
  }
}
```

### JavaScript Transform
Dynamic expression evaluated at runtime:
```json
{
  "param_name": {
    "type": "javascript",
    "expr": "results.previous_step.data"
  }
}
```

## Module Types

### RawScript
Inline script code with language specification:
- Requires `type: "rawscript"`, `language`, `content`, and `input_transforms`

### PathScript
Reference to an existing script:
- Requires `type: "script"` and `path`
- Optionally `hash` for specific version

### PathFlow
Reference to a sub-workflow:
- Requires `type: "flow"` and `path`

### ForLoopFlow
Iterate over an array:
- Requires `type: "forloopflow"`, `iterator`, and `modules`
- Access current item with `flow_input.iter.value`
- Access current index with `flow_input.iter.index`
- Optional: `parallel`, `parallelism`, `skip_failures`

### WhileLoopFlow
Loop until condition is false:
- Requires `type: "whileloopflow"` and `modules`
- Last step should return boolean to continue/stop

### BranchOne
Execute first matching branch (if/else):
- Requires `type: "branchone"`, `branches` array, and optional `default`
- Each branch has `expr` (JavaScript condition) and `modules`

### BranchAll
Execute all branches (parallel or sequential):
- Requires `type: "branchall"` and `branches` array
- Optional: `parallel` to run concurrently

### Identity
Pass-through module (no-op):
- Requires `type: "identity"`

## Resource References

- For flow inputs: Use type `"object"` with format `"resource-{type}"` (e.g., `"resource-postgresql"`)
- For step inputs: Use static value `"$res:path/to/resource"`

## Special Modules

### Failure Handler
Executes when any step fails. Has access to error details:
- `error.message` - Error message
- `error.step_id` - ID of failed step
- `error.name` - Error name
- `error.stack` - Stack trace

### Preprocessor Module
Runs before the first step, transforms flow inputs.
