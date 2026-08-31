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
- The step keeps only the flow-local inputs (`user_message`, `user_attachments`) in its own
  `input_transforms`; the brain and tools stay in the resource (read-only in the step).
- The agent carries its tools' default input bindings verbatim as authored (static, AI-filled,
  or flow expressions), so saving round-trips losslessly. Each host flow overrides what it
  needs: `tool_inputs` stores per-tool overrides (a diff from the resource tool's own
  transforms) that overlay onto the matching tools at runtime. Editing on a linked step edits
  the flow's use of the agent; editing under the "Editing" banner edits the agent itself.

In the flow editor, the AI agent step's **Step Input** tab shows a single read-only card
(*linked to <path>*, with the inherited brain + tools and an explanatory tooltip) plus
*Edit* (fork into the editable step, Save changes upserts back and re-links) and *Unlink*
(fork the resolved config — including any `tool_inputs` — back into the step as a one-off).
While editing, the step is the only copy of the edits: Cancel drops them and re-links (asking
first when there is something to drop), and the unsaved-changes badge opens a diff against the
deployed agent whose Discard changes is Cancel without the question. What a fork is an edit of,
and the deployed baseline the edits are judged against, live in `agentEditStore` (in memory), so
a reload brings the step back as a standalone agent with no path to save back to.
A linked agent's tools appear as display-only graph tool nodes (clicking one selects the
agent step); below the step's inputs, each tool gets a section with the standard schema-aware
input editors (prop picker included) and a read-only view of its code — edits persist into
`tool_inputs`.

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
