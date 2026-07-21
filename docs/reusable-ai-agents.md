# Reusable AI Agents

An AI agent flow step can be saved as a **reusable agent** — a resource of the built-in
`ai_agent` resource type that bundles the agent's brain (provider/model, system prompt,
temperature, output schema, memory…) and its tool set. Other flows can link to the same
agent, and edits to the agent propagate to every linked step.

The `ai_agent` resource type is defined in the hub (windmill-integrations) and synced into
every workspace via the standard cached-resource-type sync, like other built-in types.

## Rigid linking

`FlowModuleValue::AIAgent` has an optional `agent` field holding the resource path, plus a
`tool_inputs` map (per-tool host-flow input overrides). When `agent` is set:

- The brain config and tools are resolved at runtime from the resource
  (`windmill-worker/src/ai_executor.rs`, via `get_resource_value_interpolated` — nested
  provider `$res:` credentials resolve automatically).
- The step keeps only the flow-local inputs (`user_message`, `user_attachments`) in its own
  `input_transforms`; the brain and tools stay in the resource (read-only in the step).
- Saving an agent strips flow-context tool inputs (`flow_input`/`results` expressions become
  AI-filled), so the resource never carries references to its authoring flow. Each host flow
  rebinds what it needs: `tool_inputs` stores per-tool overrides (a diff from the resource
  tool's own transforms) that overlay onto the matching tools at runtime.

In the flow editor, the AI agent step's **Step Input** tab shows a single read-only card
(*linked to <path>*, with the inherited brain + tools and an explanatory tooltip) plus
*Edit* (fork into the editable step, Save changes upserts back and re-links) and *Unlink*
(fork the resolved config — including any `tool_inputs` — back into the step as a one-off).
A linked agent's tools appear as display-only graph tool nodes (clicking one selects the
agent step); below the step's inputs, each tool gets a section with the standard schema-aware
input editors (prop picker included) and a read-only view of its code — edits persist into
`tool_inputs`.

Sharing works through standard resource folder permissions (save agents under `f/...`).
