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
- A flow whose `failure_module` runs still ends as failed, unless the handler returns an object with `recover: true`, which makes it a success:
  - failing step at the top level: the flow ends, with the handler's result
  - inside a sequential loop or a branch: the flow stops there, with the enclosing step's output so far
  - inside a parallel loop, a parallel branchall, a `flow` step, a loop with `skip_failures: true` or a branchall branch with `skip_failure: true`: only that iteration, branch or subflow counts as a success, and the flow continues

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

- `provider` is an object, not a bare resource string: `{ "kind": <provider kind>,
  "resource": "$res:<path>", "model": <model id> }`. Required unless the module links to a saved
  agent through `value.agent`. Static is right for a flow run from a form; a chat flow wires its
  fields to flow inputs instead — see below

### Chat-Mode Flows

A flow with `value.chat_input_enabled: true` is run from a chat instead of a form: the composer
sends one message per turn and renders the conversation. It needs a required `user_message` string
input, read by the agent. Any other flow input the composer does not edit itself is asked for
under Configure inputs.

**A static `provider` gives a chat that cannot change its model.** Feed it from flow inputs
instead, either way round: one input carrying the whole object (`"expr": "flow_input.model_config"`)
makes every field editable, or wire it field by field to fix some and expose others. A field the
chat can write becomes a control in the composer — a provider picker, a model list, a thinking
control — and a field left static is fixed, with no control drawn for it. `kind` is the one
exception: the composer writes it only together with `resource`, since a provider is picked as a
pair, so a `kind` input wired on its own stays askable under Configure inputs and nothing the run
needs becomes unreachable.

```json
{
  "id": "chat_agent",
  "value": {
    "type": "aiagent",
    "input_transforms": {
      "provider": {
        "type": "javascript",
        "expr": "({ kind: 'anthropic', resource: '$res:f/ai/claude', model: flow_input.model, reasoning_effort: flow_input.thinking })"
      },
      "user_message": { "type": "javascript", "expr": "flow_input.user_message" },
      "user_attachments": { "type": "javascript", "expr": "flow_input.files" },
      "memory": { "type": "static", "value": { "kind": "compaction" } },
      "streaming": { "type": "static", "value": true },
      "output_type": { "type": "static", "value": "text" }
    },
    "tools": []
  }
}
```

- Wiring field by field means one object literal whose values are literals or bare `flow_input.x`
  references. A spread, a call or a computed key leaves the composer unable to tell which input
  feeds which field, so it offers no control at all — a bare `flow_input.x` for the whole object
  is read instead as that one input carrying every field
- `memory` is what lets the agent see earlier turns; without it every message starts from nothing
- `streaming` on makes the answer and its thinking appear token by token instead of all at once
- `user_attachments` points at a flow input typed as an array of s3 objects
  (`{ "type": "array", "items": { "type": "object", "resourceType": "s3object" } }`), so files
  sent with a message reach the agent
- Running one needs a `memory_id` **query parameter** — not a flow argument — naming the
  conversation the turn belongs to: a fresh UUID starts one, reusing a UUID continues it. The chat
  supplies it itself; a run driven any other way has to pass it or the server refuses the job

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
- A tool name outside that character set fails any run that offers the tool to the agent, with `Invalid tool name`.
<!-- chat-only -->
  The flow write tools refuse such a name.
<!-- /chat-only -->
<!-- cli-only -->
  `wmill lint <flow folder>` reports it before anything runs.
<!-- /cli-only -->
- Tool `id` follows the same rules as any module ID — unique across the flow, underscores not spaces
- `description` is optional free text telling the agent when and how to call the tool. Set it
  whenever the name alone does not make that obvious; it overrides the description derived from the
  underlying script

## AI Decision Modules

An `aidecision` module asks a decision model (TypeSafe's Jev or Cloudflare's Clef) typed questions about a `state` and
answers each with calibrated probabilities instead of text. Prefer it over an `aiagent` when the step
is a judgment (classify, route, score or a yes/no check) that needs no tools and no free text: it
is faster, cheaper, and its answers have a fixed shape.

```json
{
  "id": "triage",
  "summary": "Classify the ticket",
  "value": {
    "type": "aidecision",
    "input_transforms": {
      "provider": {
        "type": "static",
        "value": { "kind": "typesafe", "resource": "$res:f/ai/typesafe", "model": "jev-latest" }
      },
      "state": { "type": "javascript", "expr": "flow_input.message" },
      "questions": {
        "type": "static",
        "value": {
          "intent": {
            "type": "choice",
            "instructions": "What does the customer want?",
            "criteria": { "refund": "Money back", "bug": "Something is broken", "other": "Anything else" }
          },
          "urgency": {
            "type": "score",
            "instructions": "How urgent is the ticket?",
            "criteria": ["Can wait", "This week", "Today", "Right now"]
          },
          "angry": { "type": "noul", "instructions": "Is the customer angry?" }
        }
      }
    }
  }
}
```

- `provider.kind` is `typesafe` (`model` `jev-latest` unless a version is pinned) or `cloudflare`
  (`model` `clef`, or `clef-flash` for faster answers); the resource is of that same type
- `state` is the content to evaluate: usually a text, such as the message to classify. To combine
  several values, pass an object with descriptive keys holding only what the questions need
  (`({ message: flow_input.message, plan: results.get_account.plan })`); an array of strings also works
- `questions` maps each question name to `{ type, instructions, criteria }`:
  - `choice`: `criteria` maps each option to its description (up to 255 options). The answer has
    `choice`, `probabilities` and `confidence`
  - `score`: `criteria` is an ordered array of 2 to 10 level descriptions. The answer has `score`,
    `legend`, `probabilities` and `confidence`
  - `noul`: yes/no, `criteria` optionally `{ "true": ..., "false": ... }` descriptions. The answer has
    `noul`, the probability of yes from 0 to 1
- The result is `{ output, model, usage }` with `output` keyed by question name, so a later step
  reads `results.triage.output.intent.choice`, `results.triage.output.urgency.score` or
  `results.triage.output.angry.noul > 0.7`

### Branching on the Answers

To route on the answers, put a `branchone` right after the decision, one branch per option, with
each `expr` reading the decision by id:

```json
{
  "id": "route",
  "value": {
    "type": "branchone",
    "branches": [
      { "summary": "refund", "expr": "results.triage.output.intent.choice === 'refund'", "modules": [...] },
      { "summary": "bug", "expr": "results.triage.output.intent.choice === 'bug'", "modules": [...] }
    ],
    "default": [...]
  }
}
```

For a yes/no question, one branch with `"expr": "results.triage.output.angry.noul >= 0.5"` (tune the
threshold as needed) and the `default` as the "no" path.

### As an Agent Tool

An `aidecision` can be a `flowmodule` tool of an `aiagent` (`"tool_type": "flowmodule", "type":
"aidecision"`): set `state` to `{ "type": "ai" }` so the calling model supplies it, and only `output`
goes back to the model. The Tool Naming Rules apply to its `summary`. Only a flow's own agent step
can call one, not an agent used as a tool.

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

- A `forloopflow` runs its `modules` once per element of `iterator`, a javascript expression returning an array (e.g. `results.get_items`); `parallel: true` runs the iterations concurrently, and `skip_failures: true` carries on past a failed iteration
- For `whileloopflow`, break the loop with a module-level `stop_after_if`: on the loop module itself, or on an inner step (required when that step carries state via its own `results` — see below)
- `stop_after_if` is always a sibling of `id` and `value` on a flow module — never a direct key of the loop's `value` object
- `stop_after_all_iters_if` is for checks after the whole loop finishes, not the normal per-iteration break condition
- `stop_after_if` is evaluated after each iteration: on the loop module, `result` is that iteration's result (what its last step returned); on an inner step, it is that step's result
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

- A `branchone` runs the first of its `branches` whose `expr` is true, in order, and its `default` modules when none is; a `branchall` runs every branch (concurrently with `parallel: true`)
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

## Reusing Existing Scripts and Flows

Unless the user asked for new code, look for a workspace script or flow that already does a step's job before writing it, and reuse it by path instead of copying its logic into a rawscript:

- a workspace script: `type: script` with `path` (e.g. `f/folder/send_email`)
- a workspace flow, run as a subflow: `type: flow` with `path`
- a Hub script: `type: script` with a `hub/<version>/<app>/<name>` path

The step's `input_transforms` must cover the reused item's inputs, so read its input schema first.
<!-- cli-only -->
Find candidates in the local tree (a `.script.yaml` sits next to each script and holds its input schema, a `flow.yaml` in each flow folder) and on the workspace with `wmill script list` / `wmill flow list`; `wmill script get <path>` and `wmill flow get <path>` show an item's details.
<!-- /cli-only -->

## Additional Prompt for AI

A flow's input schema may carry a top-level `prompt_for_ai` string, as a script's can: its author's instructions to an AI choosing the inputs. Follow it when you pick arguments to run that flow, and keep it when you rewrite the schema.

## Organizing Flows: Groups and Notes

Groups and notes shape how a flow reads in the editor; neither changes what it does.

**Segment every non-trivial flow into groups without waiting to be asked.** Whenever a flow has more than a couple of steps, or consecutive steps form a stage ("fetch", "transform", "notify"), put them in a group, and aim for every meaningful step to belong to one. Use notes sparingly, for flow-wide information that belongs to no span of steps: the flow's purpose, key assumptions, warnings, TODOs. One note is usually enough; never label a run of steps with a note, which is what a group is for.

`value.groups` lists the groups, each spanning the steps from `start_id` to `end_id`:

- `start_id`, `end_id` (required): ids of the group's first and last step; the same id for both makes a one-step group
- `summary`: the group's title
- `note`: markdown shown under the title
- `color`: one of `yellow`, `blue`, `green`, `purple`, `pink`, `orange`, `red`, `cyan`, `lime`, `gray`, never a hex code or CSS color; leave it out and the editor picks one
- `autocollapse`: `true` shows the group collapsed by default

The editor refuses to draw a flow whose groups break any of these rules:

- `start_id` and `end_id` are steps of the same list: both top-level, or both in the same loop body or branch. A group can hold a loop or branch step whole, but cannot start outside one and end inside it
- `start_id` does not come after `end_id` in that list
- groups nest (one entirely inside another) but never partly overlap, and no two groups share both `start_id` and `end_id`
- groups hold ordinary steps only: never `preprocessor`, `failure`, `Input`, `Result`, `Trigger`, or an AI agent's tools

`value.notes` lists sticky notes, each with a unique `id`, markdown `text`, a `color` from the same list, and `type: free`. The `group` note type is deprecated; use `value.groups` instead.
<!-- cli-only -->
Give each note a `position` (`{ x, y }`) and a `size` (`{ width, height }`): the editor draws a note without them at the origin and cannot resize it. `x: -400` with `width: 275` places it beside the graph.
<!-- /cli-only -->
<!-- chat-only -->
Leave a note's `position` and `size` out and they are filled in for you.
<!-- /chat-only -->

## Final Structural Self-Check

Before finalizing a flow, verify:

- any preprocessor is in `value.preprocessor_module`
- any failure handler is in `value.failure_module`
- any approval step has module-level `suspend`
- no downstream step references inner branch step ids from outside the branch
- every AI agent flowmodule tool has a unique `summary` made only of letters, numbers and underscores
- every group starts and ends on steps of the same list, start before end, nesting without partial overlap
<!-- cli-only -->
- `wmill lint <flow folder>` reports no error
<!-- /cli-only -->

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
