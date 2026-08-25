# Windmill Flow Building Guide

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

## AI Agent Modules

An `aiagent` module runs an LLM that can call tools. Each entry of `value.tools` is a module-shaped
object with an extra `value.tool_type`: `flowmodule` for a script/flow tool, `mcp` for an MCP server
tool, `websearch` for web search.

```json
{
  "id": "support_agent",
  "summary": "AI agent for customer support",
  "value": {
    "type": "aiagent",
    "input_transforms": {
      "provider": {
        "type": "static",
        "value": { "kind": "openai", "resource": "$res:f/ai_providers/openai", "model": "gpt-4o" }
      },
      "output_type": { "type": "static", "value": "text" },
      "user_message": { "type": "javascript", "expr": "flow_input.query" },
      "system_prompt": { "type": "static", "value": "You are a helpful assistant." }
    },
    "tools": [
      {
        "id": "search_docs",
        "summary": "search_documentation",
        "description": "Search the product documentation. Use it whenever the user asks how a feature works.",
        "value": {
          "tool_type": "flowmodule",
          "type": "rawscript",
          "language": "bun",
          "content": "export async function main(query: string) { return ['doc1', 'doc2']; }",
          "input_transforms": { "query": { "type": "static", "value": "" } }
        }
      }
    ]
  }
}
```

- `provider` is a static object, not a bare resource string: `{ "kind": <provider kind>,
  "resource": "$res:<path>", "model": <model id> }`. Required unless the module links to a saved
  agent through `value.agent`

### Tool Naming Rules

These rules cover `flowmodule` tools, the ones the agent calls by name. A `websearch` tool's
`summary` is a plain label (`Web Search`), and an `mcp` tool exposes the MCP server's own tool
names, so neither is name-checked at all — leave those summaries as they are.

- A flowmodule tool's `summary` is the **name the agent calls it by**, not a human label. Put the
  human-readable explanation in `description`
- `summary` must match `^[a-zA-Z0-9_]+$`: letters, numbers and underscores only. No spaces, dashes,
  dots or accents — `search_documentation`, never `Search documentation`
- Always set `summary`. It must be unique among that agent's tools, and must not be one of the
  reserved ids (`do`, `bg`, `ctx`, `state`, `if`, `else`, `for`, `delete`, `while`, `new`, `in`,
  `failure`, `preprocessor`, `as`, `Input`, `Result`, `Trigger`)
- A tool name outside that character set is rejected: flow write tools refuse it, and a flow that
  reaches the worker with one fails every run with `Invalid tool name`
- Tool `id` follows the same rules as any module ID — unique across the flow, underscores not spaces
- `description` is optional free text telling the agent when and how to call the tool. Set it
  whenever the name alone does not make that obvious; it overrides the description derived from the
  underlying script

## Common Mistakes to Avoid

- Missing `input_transforms` - Rawscript parameters won't receive values without them
- Referencing future steps - `results.step_id` only works for steps that execute before the current one
- Duplicate module IDs - Each module ID must be unique in the flow
- AI agent flowmodule tool names with spaces - `summary` is the tool name and only accepts letters, numbers and underscores

## Data Flow Between Steps

- `flow_input.property` - Access flow input parameters
- `results.step_id` - Access output from a previous step only when that step result is in scope
- `results.step_id.property` - Access specific property from a previous step output only when that step result is in scope
- `flow_input.iter.value` - Current iteration value inside a `forloopflow`; in a `whileloopflow` it is just the iteration index (a plain number, same as `flow_input.iter.index`)
- `flow_input.iter.index` - Current loop index when inside a loop (`forloopflow` or `whileloopflow`)

## Loop Structure Rules

- For `whileloopflow`, break the loop with a module-level `stop_after_if`: on the loop module itself, or on an inner step (required when that step carries state via its own `results` — see below)
- `stop_after_if` is always a sibling of `id` and `value` on a flow module — never a direct key of the loop's `value` object
- `stop_after_all_iters_if` is for checks after the whole loop finishes, not the normal per-iteration break condition
- `flow_input.iter.value` in a `whileloopflow` is just the iteration index (same number as `flow_input.iter.index`) — it never carries state, so `flow_input.iter.value.<field>` is always undefined and a loop whose stop condition depends on it never terminates
- To carry state across iterations, a step reads its own previous-iteration result via `results.<its_own_id>` with a first-iteration fallback (e.g. `results.b ?? flow_input.start`) — but then the loop's `stop_after_if` MUST sit on that inner step, not on the loop module: a body that is exactly one plain step with the stop condition on the loop module runs on a fast path where `results.<step_id>` is null on every iteration and the loop never terminates (bodies with 2+ steps, or whose single step has its own `stop_after_if`, retry or similar, resolve `results` across iterations regardless of stop placement)
- For state that is just a counter, derive it from the index instead (e.g. `flow_input.iter.index + 1`) — that works in every configuration, including with `stop_after_if` on the loop module
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
            count:
              type: javascript
              expr: flow_input.iter.index + 1
- id: return_final_state
  value:
    type: rawscript
    input_transforms:
      final_state:
        type: javascript
        expr: results.loop_until_done[results.loop_until_done.length - 1]
```

Correct `whileloopflow` shape carrying state via `results` (stop condition on the inner step):

```yaml
- id: loop_until_done
  value:
    type: whileloopflow
    skip_failures: false
    modules:
      - id: advance_state
        stop_after_if:
          expr: result.done === true
          skip_if_stopped: false
        value:
          type: rawscript
          input_transforms:
            state:
              type: javascript
              expr: results.advance_state ?? flow_input.initial_state
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
    # iter.value is a number (the iteration index); there is no previous-iteration state
    expr: flow_input.iter.value.count
```

```yaml
input_transforms:
  final_state:
    type: javascript
    expr: results.loop_until_done
```

## Approval / Suspend Structure

An approval step is a normal **script** step (`type: rawscript` or `type: script`) that is turned into an approval by adding a module-level `suspend`. Its script calls `wmill.getResumeUrls(approver)` to generate the secret resume/cancel URLs and returns them so they can be sent to the approver(s) (Slack, email, etc.) or approved from the run page.

- `suspend` belongs on the flow module object itself, as a sibling of `id` and `value`
- Never put `suspend` inside `value`
- Do NOT use `type: identity` for an approval step. An identity step suspends but never produces the resume URLs, so approvers have no link to act on — it is not a functional approval.

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
    type: rawscript
    language: bun
    input_transforms:
      approver:
        type: static
        value: ''
    content: |
      import * as wmill from "windmill-client"

      export async function main(approver?: string) {
        const urls = await wmill.getResumeUrls(approver)
        // send urls.resume / urls.cancel to the approver(s), e.g. via Slack or email
        return urls
      }
```

Incorrect shape (suspend misplaced inside `value`):

```yaml
- id: request_approval
  value:
    type: rawscript
    suspend:
      required_events: 1
```

Incorrect shape (identity has no resume URLs — not a real approval):

```yaml
- id: request_approval
  suspend:
    required_events: 1
  value:
    type: identity
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
- every AI agent flowmodule tool has a unique `summary` made only of letters, numbers and underscores

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
