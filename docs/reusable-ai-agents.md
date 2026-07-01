# Reusable AI Agents

An AI agent flow step can be saved as a **reusable agent** — a resource of the built-in
`ai_agent` resource type that bundles the agent's brain (provider/model, system prompt,
temperature, output schema, memory…), its tool set, and an **eval suite**. Other flows can
link to the same agent, and edits to the agent propagate to every linked step.

## Hybrid linking

`FlowModuleValue::AIAgent` has an optional `agent` field holding the resource path. When set:

- The brain config and tools are resolved at runtime from the resource
  (`windmill-worker/src/ai_executor.rs`, via `get_resource_value_interpolated` — nested
  provider `$res:` credentials resolve automatically).
- The flow step keeps only the flow-local inputs (`user_message`, `user_attachments`) in its
  `input_transforms`.
- v1 boundary: a linked agent's tools come wholly from the resource (per-tool flow-context
  wiring is a fast-follow).

In the flow editor, the AI agent step's **Step Input** tab shows a bar to *Save as agent*,
*Use saved agent* (picker), or *Unlink* (snapshots the resolved config back into the step).

## Evals

A saved agent carries `evals.cases`. Each case has an input message, an optional LLM-judge
checklist, and optional deterministic assertions (`contains` / `not_contains` / `regex` /
`json_path_equals` / `output_schema_valid`). The **Evals** tab authors and runs them.

Cases run for real (they cost tokens): the backend pushes a single-step AIAgent flow-preview
job and grades the output. The judge itself runs as an inline AIAgent step with a structured
output schema (`{score, pass, summary}`), defaulting to the agent's own provider.

- `POST /w/{workspace}/ai_agents/run` — run a saved agent once on an input.
- `POST /w/{workspace}/ai_agents/eval_case` — run one eval case (execute + assertions + judge).

Sharing works through standard resource folder permissions (save agents under `f/...`).
