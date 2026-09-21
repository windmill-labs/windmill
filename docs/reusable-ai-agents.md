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
  (`windmill-worker/src/ai_executor.rs`): the brain is interpolated, so a nested provider `$res:`
  credential resolves automatically.
- The step keeps only the flow-local inputs (`user_message`, `user_attachments`, `enabled_tools`,
  and the history inputs `memory_id` and `previous_messages`) in its own `input_transforms`; the
  brain and tools stay in the resource (read-only in the step). `enabled_tools` says which of the
  roster this step may call, narrowing one use of a shared agent without touching the agent: an
  absent field carries every tool, a list carries the ones it names, and an empty list carries
  none.
- The agent carries its tools' default input bindings verbatim as authored (static, AI-filled,
  or flow expressions), so saving round-trips losslessly. Each host flow overrides what it
  needs: `tool_inputs` stores per-tool overrides (a diff from the resource tool's own
  transforms) that overlay onto the matching tools at runtime. Editing on a linked step edits
  the flow's use of the agent; editing in the agent editor edits the agent itself.

In the flow editor, the AI agent step's **Step Input** tab shows a single read-only card
(*linked to <path>*, with the inherited brain + tools and an explanatory tooltip) plus
*Edit*, which opens the agent editor over the flow, and *Unlink* (fork the resolved config —
including any `tool_inputs` — back into the step as a one-off).
A linked agent's tools appear as display-only graph tool nodes (clicking one selects the
agent step); below the step's inputs, each tool gets a section with the standard schema-aware
input editors (prop picker included) and a read-only view of its code — edits persist into
`tool_inputs`.

## Memory

Memory is split between three owners, so a saved agent carries whether it remembers and never
which memory it is:

- **Agent: managed memory.** `memory` is a brain key, so it moves with a saved agent.
  `{ kind: window, context_length }` has Windmill store the conversation and replay its last N
  messages; `{ kind: compaction }` replays all of it and summarizes the older part as it fills the
  model's context window (see below); `{ kind: off }` keeps none. An absent `memory` means off, the
  default: the editor turns it on when chat input is enabled, as compaction, since a chat has no
  end. `auto` and `manual` are the older spellings and are still read.
- **Run: memory id.** `flow_status.memory_id`, set when the run is queued: the chat conversation
  id, an app chat session id, or the `memory_id` run parameter. Any string is accepted, and one
  that is not a uuid is hashed to a v5 uuid scoped to the workspace and the flow the run started
  from (`memory_key` in `windmill-common/src/flow_conversations.rs`), so the same key in two flows
  names two memories. A uuid is used as is. Nothing is generated at save time, so schedules,
  webhooks, evals and plain runs pass no id and run stateless.
- **Step: history inputs.** Flow-local, so they stay on a linked step. Each is read in one memory
  state only, and the editor offers it only there, the memory id behind a *Custom* toggle that
  writes the key only once it is on. With managed memory on, `memory_id` overrides the run's id,
  hashed the same way: a fixed value is one memory shared by every run, an expression such as
  `flow_input.customer_id` one memory per key, and an expression that evaluates to nothing runs
  stateless rather than falling back to the run's id. With memory off, `previous_messages` supplies
  the history itself. An older `auto` or `manual` memory reads neither, so the editor offers them
  only once the step is moved to the current settings, which the alert's button does. The editor
  never seeds a placeholder for either, because a present key is the step's choice, and a static
  empty value reads as unset.

The worker reconciles them once per agent invocation, nested agent tools included, in
`resolve_history_source` (`windmill-worker/src/ai_executor.rs`):

1. A legacy `auto` or `manual` memory: read as the editor that wrote it ran it. `manual` replays
   its list; `auto` uses the run's memory id, else the id baked into it, else runs stateless.
   Neither history input is read. An `auto` without a count, or with 0, is off and read as such.
2. Managed memory, `window` or `compaction`: the memory id is the step's, else the run's. With no
   memory id the agent runs stateless, and a step `previous_messages` is ignored. Compaction still
   bounds a stateless run's own loop, which is where a long tool sequence overflows.
3. Memory off: the history is `previous_messages`, else nothing. Memory is neither read nor
   written, and a step `memory_id` is ignored.

Each ignored input and each stateless fallback is written to the job log.

### Compaction

`{ kind: compaction }` keeps the whole conversation and lets a summary, rather than a message count,
decide what leaves the prompt.

The window it plans against comes from the model, through `MODEL_CONTEXT_WINDOWS` in
`windmill-ai/src/model_context.rs`, falling back to 128000 for an id the table does not list. That
table mirrors the one the AI session's own compaction reads
(`frontend/src/lib/components/copilot/modelConfig.ts`) and the two have to be updated together. The
step's `context_window` overrides it, for a Custom AI deployment or a model the table cannot name;
setting it too large never trips the trigger and the provider raises the context error itself.

`windmill-worker/src/ai/compaction.rs` keeps two histories: model context, which can be compacted,
and the execution record, which retains the loaded history and every message produced by the run.
Returned results and partial-error results use the execution record. Compaction cannot remove or
reorder the action messages the flow viewer indexes, or the MCP results it finds by call ID.

Before each provider request, including the first, the worker checks the projected context size.
At 80% of the model window it summarizes the older prefix, keeping recent complete exchanges
verbatim. The soft target is 50%, including the system prompt, tools and summary allowance. The
newest exchange is always retained even if it exceeds that target. A user prompt stays with its
first response; later tool rounds can be compacted within a single turn, but a call and its results
are never split. A prefix containing only an earlier summary is not summarized again.

The summarization request carries no tools; its tool exchanges are rendered as text because
Bedrock rejects tool blocks without definitions. A dedicated system instruction asks for a factual
handoff from a labelled transcript, keeping the compaction instruction outside that transcript;
media parts remain available. Its output cap matches the reserved summary budget,
and its temperature and reasoning settings are independent of the step's answer settings.
If summarization fails, history is retained while it fits. Older complete exchanges are omitted
only when the estimated model limit or exact storage limit is exceeded. Three consecutive failures
disable summary requests for the run. Before a provider request, an irreducible oversized exchange
returns a capacity error with the execution record. After the answer, an unsavable context skips
the memory write and logs that the next run will load the previous saved memory; the answer succeeds.
Flow logs distinguish summaries, omissions and skipped writes.

The projection uses normalized `TokenUsage::input_tokens` from the last request plus a `bytes/4`
estimate of appended messages. Without usage, or after rewriting context, it estimates the whole
prompt including tools. S3 descriptors get a nominal attachment allowance; actual attachment costs
and tokenizer differences remain approximate. Provider parsing owns usage normalization: Anthropic
and Bedrock report cached input separately, while OpenAI-shaped providers include it in input tokens.
If a provider explicitly rejects the context size, the worker summarizes all older complete
exchanges (or omits them if summarization fails) and retries that request once. This also recovers
from undercounted attachments loaded from a previous run, without provider-specific tokenizers.
Unrelated errors are not retried this way. The newest exchange is still retained, so a request
that cannot fit even after recovery fails; estimates do not guarantee every first request fits.

Memory is stored per (memory id, step id), in `ai_agent_memory` or S3 at
`memory/{workspace}/{memory id}/{step}.json`. The chat transcript (`flow_conversation_message`)
always follows the run's id, even when a step sets its own. Before persistence, the same planner
also checks the serialized memory against the database's 100KB limit (`MAX_MEMORY_SIZE_BYTES`),
when `memory_storage_capacity_bytes` reports one. System messages and tool definitions are not
stored, so they do not count towards this byte limit. The final pass chooses one prefix against
both constraints and summarizes it once; it does not shrink the model window to a storage-derived
token count. The actual replacement is checked again before writing. Persistence reads compacted
model context, independently of the complete execution record returned by the step.
Nothing expires stored memory: deleting a chat conversation deletes its memory, and a memory named
by a string id stays until it is overwritten.

Compatibility runs one way. New workers read every older shape. The editor rewrites a legacy step
only when the author changes it, so a flow nobody edits keeps running on older workers, while a
step saved with `window`, `compaction` or a history input needs a worker that knows them. An id an
older editor baked into `memory` stays a fallback behind the run's id until the author chooses
*Keep as memory id* or *Use the run's memory id*. In a chat flow it is dropped on save, since the
conversation id always took precedence there.

## Drafts

The agent editor edits the resource through a **per-user resource draft** (`draft` table,
`item_kind = 'resource'`), autosaved by `useAgentDraft` and deployed by the editor's own Deploy
button. It is the same draft row the generic resource editor writes and the Review & Deploy page
lists, so an agent can be deployed from any of them.

A flow does not wait for that deploy to see the draft:

- Testing the flow, or a single linked step, runs the draft. `runFlowPreview` and `ModuleTest`
  substitute each linked step for the standalone step the draft would run as
  (`linkedAgentDrafts.ts`): `agent` cleared, the draft's brain as static input transforms, the
  draft's tools on the step, and the step's own flow-local inputs kept on top —
  the same overlay order `ai_executor.rs` applies to a linked step. `tool_inputs` is untouched,
  since the worker overlays it in both branches.
- The step's linked card and the graph's tool nodes show the draft, with a *Draft* badge, so the
  editor describes what a test would run. Read-only surfaces (the deployed flow page, the run
  viewer) stay on the deployed agent: they resolve tools through `publishLinkedAgentTools` without
  the draft flag.
- Deploying the flow lists every linked agent that has a draft in the confirmation dialog, beside
  the draft triggers. Deploying one writes the resource and drops the draft; leaving one out keeps
  its draft untouched, and the flow runs the agent as currently deployed. That is the one place
  the two kinds differ: an undeployed draft trigger is deleted, because it belongs to the flow,
  while an agent draft belongs to a resource other flows also use.

Because a draft is per-user, a flow test can behave differently for two people looking at the same
flow. That is the same contract as a flow draft, and deploying the agent is what makes it shared.

Inlining has a consequence worth knowing: a preview job's `raw_flow` then carries the agent's
config, where a linked step used to carry only the path and leave the resolution to the worker. So
an agent's prompt and tool set are readable by whoever can read that preview job, which is a wider
set than whoever can read the resource when the agent sits in a more restricted folder than the
flow. No credential travels with it — the provider stays a `$res:` reference, resolved at run time
as the runner. The agent editor's own test pane has inlined the same way since drafts existed;
closing the gap would mean the preview carrying a draft *reference* the worker resolves, rather
than the config.

Sharing works through standard resource folder permissions (save agents under `f/...`).

Only the agent's brain is interpolated when the step runs. A tool's own `$res:`/`$var:` defaults are
left alone and resolved when that tool executes, so a host flow can override a default pointing at a
resource it cannot read — and an unused tool whose default is inaccessible never fails the agent.

## Resolution is live, not pinned

A linked step resolves its agent resource when the step runs — that is what makes an edit propagate
to every linked flow. It also means a run is not a snapshot: editing the agent while a flow is
in-flight affects steps that have not started yet. The same applies one level down, where the effect
is sharper: a *nested* agent tool of a linked agent runs as its own job and looks its definition up
in the resource again by tool id, so an edit landing between the LLM selecting that tool and the
tool starting can run the changed definition, or fail if the tool was removed. Pinning would require
carrying the resolved definition into the child job rather than its id. Inline (unlinked) agents are
unaffected: their tools live in the flow value, which is snapshotted with the run.

## Version history

Editing a resource appends a row to `resource_version` (all types except `state` and `cache`,
which the platform rewrites on every job), so an agent's prompt, model and tool set can be
diffed and restored from the resource editor's History drawer. Restoring writes the old value
forward as a new version rather than rewinding, keeping the history append-only.

History captures the resource, not its transitive closure. A `$var:`/`$res:` reference is stored
as the reference, so two versions can be byte-identical while the agent behaves differently
because the referenced variable changed underneath them. Anything comparing agent runs across
versions has to account for that.

An eval run records the version its agent was at when the run was enqueued, which is what makes a
result attributable to a prompt state — see `docs/ai-agent-evals.md`.

A superseded value is retained for up to 100 versions. Values written through the UI keep their
secrets in linked variables, but one pushed by `wmill` or written by `setResource` can hold an
inline credential, and overwriting it no longer removes it from the database — anyone who can
read the resource can read it from the history. Rotating such a credential therefore does not
erase the old one on its own — follow the rotation with **Clear past versions** in the History
drawer, which drops every version but the current value for that one resource. Secret *variables*
are deliberately not versioned at all for the same reason.

## Dependencies and locks

A linked step carries `tools: []`, and no dependency job ever visits the `ai_agent` resource, so the
tool scripts inside it are outside the lockfile and dependency-map pipelines: `lock_modules` has
nothing to lock on the step, and `FlowValue::traverse_leafs` sees no leaf for them. Consequences:

- Raw-script tools saved into an agent keep whatever `lock` they had on the authoring step (`null`
  if that step was never deployed), and every linked flow resolves their dependencies at job time.
- Script tools referenced by path are invisible to redeploy cascades — republishing such a script
  does not re-lock the flows that link the agent.

Deploying a linked flow to another workspace pulls the `ai_agent` resource in as a dependency, and
from there its provider resource and its tools' scripts, flows, MCP resources and nested agents.
