# Global AI Chat Mode POC Plan

## Goal

Add a POC "global" AI chat mode that can reason across workspace items and create draft changes for scripts, flows, and apps. For this POC, write and modify tools must not call save/update APIs. They should write to a frontend global draft store that can later feed the corresponding editor or viewer.

## Non-Goals For POC

- Do not persist AI-created or AI-modified items to the backend.
- Do not implement resources, triggers, schedules, variables, or datatables yet.
- Do not build a full diff/review UI beyond exposing enough draft state for follow-up rendering.
- Do not replace existing script, flow, app, app-mode, or API-mode chat behavior.

## Core Design

Global mode should be a new AI chat mode with a small curated tool surface:

1. `get_instructions`
   - Returns focused instructions for a specific subject.
   - Initial subjects: `script`, `flow`, `app`.
   - The global mode system prompt should instruct the model to call this before writing or modifying that subject type.

2. `list_workspace_items`
   - Lists current workspace scripts, flows, apps, and AI drafts.
   - Returns metadata only: `type`, `path`, `summary`, draft status, and size/version hints.
   - Does not include source code, flow JSON, or raw app bundle content.

3. `read_workspace_item`
   - Reads one item by `type` and `path`.
   - If a draft exists, returns the draft and its base metadata.
   - Otherwise reads from the workspace through generated service clients.
   - Initial supported types: `script`, `flow`, `app`.

4. `write_workspace_item`
   - Creates a new draft item in the global AI draft store.
   - Fails if the item exists in the real workspace or the draft store unless an explicit overwrite flag is provided.
   - Does not call backend create/update APIs.

5. `modify_workspace_item`
   - Creates or updates a draft overlay for an existing item.
   - Requires the existing item to be read first so the draft can keep base metadata.
   - Does not call backend create/update APIs.

## Draft Store Shape

Create a frontend store for AI workspace drafts. Suggested file:

- `frontend/src/lib/components/copilot/chat/global/draftStore.svelte.ts`

Suggested draft model:

```ts
type GlobalDraftItemType = 'script' | 'flow' | 'app'

type GlobalDraftStatus = 'new' | 'modified'

type GlobalDraftBase = {
	source: 'workspace'
	version?: string | number
	editedAt?: string
	value: unknown
}

type GlobalDraftItem = {
	type: GlobalDraftItemType
	path: string
	status: GlobalDraftStatus
	base?: GlobalDraftBase
	draft: unknown
	updatedAt: string
}
```

Store operations:

1. `listDrafts()`
2. `getDraft(type, path)`
3. `setNewDraft(type, path, draft, overwrite?)`
4. `setModifiedDraft(type, path, base, draft)`
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
   - call `get_instructions` before writing/modifying scripts, flows, or apps;
   - clearly state that write/modify creates drafts only.

### Step 2: Add Draft Store

1. Create `global/draftStore.svelte.ts`.
2. Store drafts keyed by `${type}:${path}`.
3. Preserve base metadata for modified drafts.
4. Keep draft operations synchronous and frontend-only.
5. Add small utility functions for item keys and normalized timestamps.

### Step 3: Implement Instruction Tool

1. Add `get_instructions` with a Zod schema:
   - `subject: 'script' | 'flow' | 'app'`
2. Return concise, practical instructions:
   - script: language, schema, main function expectations, path conventions;
   - flow: `OpenFlow` shape, modules, IDs, summaries, validation expectations;
   - app: raw app bundle shape, frontend files, backend runnables, data config.
3. Keep detailed examples short. This tool can grow later into a workspace skill system.

### Step 4: Implement Listing Tool

1. Add `list_workspace_items`.
2. Fetch workspace items with generated clients:
   - `ScriptService.listScripts`
   - `FlowService.listFlows`
   - `AppService.listApps`
3. Merge drafts from the draft store into the result.
4. Mark each result:
   - `source: 'workspace' | 'draft' | 'workspace+draft'`
   - `draftStatus?: 'new' | 'modified'`
5. Return compact metadata only.

### Step 5: Implement Read Tool

1. Add `read_workspace_item`.
2. If a draft exists, return draft state first.
3. If no draft exists:
   - scripts: `ScriptService.getScriptByPath`
   - flows: `FlowService.getFlowByPath`
   - apps: `AppService.getAppByPath`
4. Normalize output with:
   - `type`
   - `path`
   - `source`
   - `baseVersion`
   - `value`
5. For raw apps, initially read app metadata/value. Defer full raw bundle hydration if the exact existing app API shape needs more investigation.

### Step 6: Implement Write Draft Tool

1. Add `write_workspace_item`.
2. Validate item type and path.
3. Check for conflicts:
   - existing draft with same key;
   - existing workspace item with same type/path.
4. If conflict exists and `overwrite` is not true, return a clear error.
5. Store a `status: 'new'` draft.
6. Return a short summary that explicitly says the workspace was not saved.

### Step 7: Implement Modify Draft Tool

1. Add `modify_workspace_item`.
2. Require existing draft or existing workspace item.
3. If no draft exists, fetch workspace item and store it as `base`.
4. Store a `status: 'modified'` draft with the new draft value.
5. Return base version/hash metadata and a clear "draft only" message.
6. Consider requiring `baseVersion` in a follow-up after the POC to reduce stale edits.

### Step 8: Wire UI Entry Point

1. Add Global mode wherever AI modes are selectable.
2. Ensure opening global mode does not clear app/script/flow editor-specific helper state unexpectedly.
3. Label the mode clearly as draft-based, for example `Workspace` or `Global`.
4. Make draft write/modify tool execution visible in the existing tool display.

### Step 9: Prepare For Editor Handoff

1. Add helper selectors in the draft store:
   - `getScriptDraft(path)`
   - `getFlowDraft(path)`
   - `getAppDraft(path)`
2. Do not wire full editor hydration in this POC unless trivial.
3. Document expected future handoff:
   - script draft opens ScriptBuilder with draft content;
   - flow draft opens FlowBuilder with draft `OpenFlow`;
   - app draft opens raw app editor with draft files/runnables/data.

### Step 10: Validation

Since this POC is frontend chat code:

1. Run `npm run check:fast` from `frontend/` during iteration.
2. Run `npm run check` from `frontend/` before finalizing.
3. Manually test in the running frontend:
   - switch to global/workspace mode;
   - list scripts, flows, apps;
   - read one script, one flow, one app;
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
4. Add resources, triggers, schedules, variables, and datatables.
5. Add permission-aware read filtering and secret redaction.
6. Add backend-backed apply/deploy flows through existing editors, not directly from the global tool.
