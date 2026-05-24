# Live Editor Effective Path Registry

## Goal

Let global chat target the currently open editor without making the LLM understand empty `UserDraft` paths.

Instead of treating `path: ""` as special, editors publish a tiny live-editor record:

```ts
type LiveEditorDraft = {
	kind: UserDraftItemKind
	storagePath: string
	effectivePath?: string
}
```

The registry is per frontend runtime. We can assume one active live editor per editor kind unless the UI later supports multiple simultaneous editors of the same kind.

## Semantics

- `storagePath` is the actual `UserDraft` key path.
  - For new items this may be `""`.
  - Tools use this to read, write, or discard the draft.
- `effectivePath` is the user-facing workspace path.
  - Tools use this in status text, confirmations, returned JSON, and follow-up context.
  - It may be absent for editors where the user has not chosen a path yet.

Existing global chat tools can then add an explicit argument:

```ts
use_current_live_item?: boolean
```

When this is true, the tool resolves the target from the registry:

```ts
const target = use_current_live_item
	? liveEditorDraftRegistry.get(kind)
	: { storagePath: path, effectivePath: path }
```

This keeps the LLM instruction simple:

```text
When the user refers to the currently open editor, set use_current_live_item: true.
Do not guess paths or use path: "".
```

## Why This Helps

- No `UserDraft` rekeying is required.
- No global chat adapter needs to infer paths from item-specific draft values.
- Empty storage keys stay an implementation detail.
- The tool can validate that a live editor exists before reading or writing.
- The same path-based tools still work for closed drafts and deployed workspace items.

## Script Registry Difficulty

Difficulty: low.

Relevant current state:

- `scripts/add/+page.svelte` uses `UserDraft.use<Script>('script', '', ...)`.
- `scripts/edit/[...path]/+page.svelte` uses `UserDraft.use<EditableScript>('script', draftPath)`.
- The effective path is already in `scriptHandle.draft.path`.

Implementation shape:

- Register `{ kind: 'script', storagePath: '', effectivePath: scriptHandle.draft?.path }` from the add route.
- Register `{ kind: 'script', storagePath: draftPath, effectivePath: scriptHandle.draft?.path ?? draftPath }` from the edit route.
- Update the registry reactively when `scriptHandle.draft.path` changes.
- Unregister on route/component destroy.

Risks:

- Need to avoid registering an empty effective path before the auto-assigned path exists.
- Hash/fork edit routes can also use `storagePath: ''`; the registry must preserve that storage path while exposing the draft's real path when present.

Expected size:

- Small registry module plus route effects.
- No editor content plumbing needed.

## Flow Registry Difficulty

Difficulty: medium.

Relevant current state:

- `flows/add/+page.svelte` uses `UserDraft.use<Flow>('flow', '', ...)`.
- `flows/edit/[...path]/+page.svelte` uses `UserDraft.use<Flow>('flow', flowDraftPath)`.
- The effective path is not always `flowHandle.draft.path`.
- `FlowBuilder` keeps the live edited path in `pathStore`.

Implementation shape:

- Register from `FlowBuilder` or pass the current `pathStore` back to the route.
- For add route, publish `{ kind: 'flow', storagePath: '', effectivePath: $pathStore }`.
- For edit route, publish `{ kind: 'flow', storagePath: flowDraftPath, effectivePath: $pathStore || flowDraftPath }`.
- Update reactively when `$pathStore` changes.
- Unregister on destroy.

Risks:

- Registration likely belongs near `FlowBuilder`, because that is where `pathStore` is authoritative.
- Flow has more side state than scripts: draft triggers, selected module state, flow history, and path changes from fork/hub/template flows.
- The registry should only publish identity, not attempt to own flow content.

Expected size:

- Moderate. The registry itself is small, but wiring it cleanly through `FlowBuilder` or route bindings needs care.

## Raw App Registry Difficulty

Difficulty: medium to high.

Relevant current state:

- `apps_raw/add/+page.svelte` uses `UserDraft.use('raw_app', '')`.
- `apps_raw/edit/[...path]/+page.svelte` uses `UserDraft.use('raw_app', path)`.
- Raw app draft value is `{ files, runnables, data, summary }`; it does not contain the workspace path.
- The effective path lives in `RawAppEditorHeader` as `newEditedPath`.

Implementation shape:

- Register from the component boundary that can see both:
  - the route/storage path
  - `RawAppEditorHeader`'s `newEditedPath`
- For add route, publish `{ kind: 'raw_app', storagePath: '', effectivePath: newEditedPath }` when non-empty.
- For edit route, publish `{ kind: 'raw_app', storagePath: path, effectivePath: newEditedPath || path }`.
- If the path is not known yet, either omit `effectivePath` or report that the current raw app draft has no path selected.

Risks:

- Path state is lower in the component tree than the route-owned `UserDraft` handle.
- The save/header UX controls path selection, so the registry may need a new prop/event to lift `newEditedPath`.
- There may be a period where there is a live raw app draft but no effective path.

Expected size:

- More than scripts and flows. It likely needs prop/event plumbing between the raw app route/editor/header.

## Tool Behavior With The Registry

For tools that accept `use_current_live_item`:

1. Resolve the registry entry for the requested kind.
2. Error if no live editor of that kind is registered.
3. Error if the tool needs a path but `effectivePath` is missing.
4. Read/write the draft using `storagePath`.
5. Return `effectivePath` in tool output and status text.

This keeps empty-path handling out of the prompt and out of model reasoning.

## Recommendation

Start with scripts, then flows, then raw apps.

- Scripts validate the registry shape with low risk.
- Flows prove that the registry can solve the `pathStore` problem.
- Raw apps should come after the registry API is stable because their path state is split across more components.
