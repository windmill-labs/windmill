# Global AI Chat Mode POC Plan

## Goal

Add a POC "global" AI chat mode that can reason across workspace scripts and flows and create draft changes for them. For this POC, write and modify tools must not call save/update APIs. They should write to a frontend global draft store that can later feed the corresponding editor or viewer.

## Non-Goals For POC

- Do not persist AI-created or AI-modified items to the backend.
- Do not implement resources, triggers, schedules, variables, or datatables yet.
- Do not build a full diff/review UI beyond exposing enough draft state for follow-up rendering.
- Do not replace existing script, flow, app-mode, or API-mode chat behavior.

## Core Design

Global mode should be a new AI chat mode with a small curated tool surface:

1. `get_instructions`
   - Returns focused instructions for a specific subject.
   - Initial subjects: `script`, `flow`.
   - The global mode system prompt should instruct the model to call this before writing or modifying that subject type.

2. `list_workspace_items`
   - Lists current workspace scripts, flows, and AI drafts.
   - Returns metadata only: `type`, `path`, `summary`, `language`, and `isDraft`.
   - Does not include source code or flow JSON.

3. `read_workspace_item`
   - Reads one item by `type` and `path`.
   - If a draft exists, returns the draft item.
   - Otherwise reads from the workspace through generated service clients.
   - Initial supported types: `script`, `flow`.

4. `write_workspace_item`
   - Creates a new draft item in the global AI draft store.
   - Fails if the item exists in the real workspace or the draft store unless an explicit overwrite flag is provided.
   - Does not call backend create/update APIs.

5. `modify_workspace_item`
   - Creates or updates a draft overlay for an existing item.
   - Requires the existing item to be read first so the model can produce a complete replacement value.
   - Does not call backend create/update APIs.

## Draft Store Shape

Create a frontend store for AI workspace drafts. Suggested file:

- `frontend/src/lib/components/copilot/chat/global/draftStore.svelte.ts`

Suggested item/draft model:

```ts
type GlobalWorkspaceItemType = 'script' | 'flow'

type GlobalWorkspaceItem = {
	type: GlobalWorkspaceItemType
	path: string
	summary?: string
	value: unknown // script source code or OpenFlow value
	language?: ScriptLang // required for scripts, omitted for flows
	isDraft: boolean
}
```

Store operations:

1. `listDrafts()`
2. `getDraft(type, path)`
3. `setNewDraft(item, overwrite?)`
4. `setModifiedDraft(item)`
5. `deleteDraft(type, path)`
6. `clearDrafts()`

## Implementation Steps

### Step 1: Add Global Mode Skeleton

1. Add a new `AIMode.GLOBAL` entry in `AIChatManager.svelte.ts`.
2. Add mode wiring in `AIChatManager.changeMode`.
3. Create `frontend/src/lib/components/copilot/chat/global/core.ts`.
4. Export:
   - `prepareGlobalSystemMessage(customPrompt?: string)`
   - `prepareGlobalUserMessage(instructions: string)`
   - `globalTools`
5. Keep the first system prompt short and demand-driven:
   - list before broad reads;
   - read before modify;
   - call `get_instructions` before writing/modifying scripts or flows;
   - clearly state that write/modify creates drafts only.

### Step 2: Add Draft Store

1. Create `global/draftStore.svelte.ts`.
2. Store drafts keyed by `${type}:${path}`.
3. Store drafts as simple `GlobalWorkspaceItem` values with `isDraft: true`.
4. Keep draft operations synchronous and frontend-only.
5. Add small utility functions for item keys.

### Step 3: Implement Instruction Tool

1. Add `get_instructions` with a Zod schema:
   - `subject: 'script' | 'flow'`
2. Return concise, practical instructions:
   - script: language, source-code value, main function expectations, path conventions;
   - flow: `OpenFlow` value shape, modules, IDs, summaries, validation expectations.
3. Keep detailed examples short. This tool can grow later into a workspace skill system.

### Step 4: Implement Listing Tool

1. Add `list_workspace_items`.
2. Fetch workspace items with generated clients:
   - `ScriptService.listScripts`
   - `FlowService.listFlows`
3. Merge drafts from the draft store into the result.
4. Mark drafts with `isDraft: true`.
5. Return compact metadata only without `value`.

### Step 5: Implement Read Tool

1. Add `read_workspace_item`.
2. If a draft exists, return draft state first.
3. If no draft exists:
   - scripts: `ScriptService.getScriptByPath`
   - flows: `FlowService.getFlowByPath`
4. Normalize output as `{ type, path, summary, value, language, isDraft }` where `value` is script source code or the OpenFlow value.

### Step 6: Implement Write Draft Tool

1. Add `write_workspace_item`.
2. Validate item type and path.
3. Check for conflicts:
   - existing draft with same key;
   - existing workspace item with same type/path.
4. If conflict exists and `overwrite` is not true, return a clear error.
5. Store the draft as a `GlobalWorkspaceItem` with `isDraft: true`.
6. Return a short summary that explicitly says the workspace was not saved.

### Step 7: Implement Modify Draft Tool

1. Add `modify_workspace_item`.
2. Require existing draft or existing workspace item.
3. Store the modified item as a draft with `isDraft: true`.
4. Return a clear "draft only" message.
5. Consider adding base version/hash checks in a follow-up after the POC to reduce stale edits.

### Step 8: Wire UI Entry Point

1. Add Global mode wherever AI modes are selectable.
2. Ensure opening global mode does not clear script/flow editor-specific helper state unexpectedly.
3. Label the mode clearly as draft-based, for example `Workspace` or `Global`.
4. Make draft write/modify tool execution visible in the existing tool display.

### Step 9: Prepare For Editor Handoff

1. Add helper selectors in the draft store:
   - `getScriptDraft(path)`
   - `getFlowDraft(path)`
2. Do not wire full editor hydration in this POC unless trivial.
3. Document expected future handoff:
   - script draft opens ScriptBuilder with draft content;
   - flow draft opens FlowBuilder with draft `OpenFlow`.

### Step 10: Validation

Since this POC is frontend chat code:

1. Run `npm run check:fast` from `frontend/` during iteration.
2. Run `npm run check` from `frontend/` before finalizing.
3. Manually test in the running frontend:
   - switch to global/workspace mode;
   - list scripts and flows;
   - read one script and one flow;
   - create a new draft script;
   - modify an existing item into a draft;
   - verify no backend create/update API was called for write/modify.

## Suggested File Changes

- `frontend/src/lib/components/copilot/chat/AIChatManager.svelte.ts`
- `frontend/src/lib/components/copilot/chat/global/core.ts`
- `frontend/src/lib/components/copilot/chat/global/draftStore.svelte.ts`
- Any mode selector/display component that enumerates `AIMode`
- Focused frontend tests if existing chat tool tests can cover draft-store behavior

## Follow-Up After POC

1. Add draft review and diff UI.
2. Add editor handoff actions.
3. Add draft persistence if users need drafts to survive reloads.
4. Add other workspace item types if needed later.
5. Add resources, triggers, schedules, variables, and datatables.
6. Add permission-aware read filtering and secret redaction.
7. Add backend-backed apply/deploy flows through existing editors, not directly from the global tool.
