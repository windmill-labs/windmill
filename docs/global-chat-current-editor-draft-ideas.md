# Global Chat Current Editor Draft Ideas

## Problem

Global chat needs to operate on the item the user is actively editing, including new items whose `UserDraft` storage key may still be empty:

```text
userdraft/w/{workspace}/{kind}/
```

The user-facing path may live somewhere else:

- in the draft value, such as `script.path`
- in editor state outside the draft value, such as a flow builder path store
- in save/header UI that is not part of the draft value, such as raw app path state

This makes path-based tools hard to use reliably for "the current editor".

## Idea 1: Current Editor Tools

Add global chat tools that target the active editor session directly:

- `get_current_editor`
- `read_current_editor`
- `write_current_editor`
- `discard_current_editor_draft`

These tools would not require the model to know the workspace path. They would call the active editor's registered read/write bridge or AI helper.

Benefits:

- Best fit for the common user intent: "edit the thing I have open".
- Avoids path guessing and empty-key handling for active editors.
- Can work for scripts, flows, apps, and raw apps even when their effective path is not in `UserDraft`.

Tradeoffs:

- Only works while an editor is open and registered.
- Path-based tools are still needed for closed drafts, deployed workspace items, and broad listing.
- Needs a clear error when no editor is active or multiple editors are active.

## Idea 2: Current Editor Registry

Add a small frontend registry for active editors:

```ts
type CurrentEditorDraft = {
	workspace: string
	kind: UserDraftItemKind
	storagePath: string
	effectivePath?: string
	editorId: string
	read(): unknown
	write(value: unknown): void
	discard(): void
}
```

Editors would register on mount, update `effectivePath` as path UI changes, and unregister on destroy.

Benefits:

- Keeps `UserDraft` mostly as storage.
- Lets global chat list or target live editors without decoding item-specific state.
- Handles flow's separate path store cleanly because FlowBuilder can register the real live path.

Tradeoffs:

- Adds another live source of truth.
- Needs policies for multiple tabs/editors and stale unregisters.

## Idea 3: `UserDraft` Effective Path

Extend `UserDraft.use` or `UserDraft.useMany` with an optional effective path callback:

```ts
UserDraft.use('flow', '', {
	defaultValue: emptyFlow(),
	effectivePath: () => $pathStore
})
```

Then `UserDraft.list()` could return both:

```ts
{
	path: effectivePath,
	storagePath: '',
	live: true
}
```

Benefits:

- Centralizes the "storage path vs user-facing path" distinction.
- Makes listing live editor drafts path-aware without changing localStorage keys.
- Avoids global chat adapter heuristics like reading `value.path`.

Tradeoffs:

- Makes `UserDraft` more editor-aware.
- Still needs a separate strategy for persisted-only empty-path drafts after reload.

## Idea 4: Path-Key New Drafts Plus Last-New Pointer

Move new-item drafts to path-keyed storage as soon as the path is known:

```text
userdraft/w/{workspace}/script/u/admin/foo
userdraft/lastNew/{workspace}/script = u/admin/foo
```

The path-keyed entry makes global listing natural, while `lastNew` preserves reload restoration for `/scripts/add` or `/flows/add`.

Benefits:

- Cleaner long-term storage model.
- `UserDraft.list()` naturally returns user-facing paths.
- Global chat and other non-editor consumers become simpler.

Tradeoffs:

- Needs migration for existing empty-path drafts.
- Needs collision policy when the user types a path that already has a draft.
- Riskier for script and flow add pages because the empty key currently doubles as the reload pointer.
- Does not fit `app` and `raw_app` until their path UX exposes the path earlier.

## Idea 5: Global Chat Context Injection

Inject active route/editor context into global chat messages:

```json
{
	"currentEditor": {
		"kind": "flow",
		"path": "u/admin/foo",
		"hasLocalDraft": true
	}
}
```

Benefits:

- Cheap immediate improvement.
- Helps the model resolve "current" without listing.

Tradeoffs:

- Not sufficient if tools still cannot read/write the current draft by that path.
- Works best with current-editor tools or `UserDraft` effective paths.

## Is "Empty Path Means Current Item" Enough?

It can be a temporary instruction, but it should not be the main design.

Problems:

- Empty path is a storage slot, not a stable item identity.
- It is only unique per workspace and kind, not per browser tab or editor instance.
- It means "new-item scratch draft", not necessarily "the currently active editor".
- It does not tell the model the user-facing path for confirmations, deploys, links, or follow-up reads.
- Some editors keep the real path outside the draft value, so empty path cannot recover it.
- Passing `path: ""` to generic read/write tools can create or update an invalidly addressed draft instead of the intended workspace item.
- Closed stale empty-path drafts would be indistinguishable from a live current editor unless the tool surface checks active editor state.

A safer stopgap instruction would be:

```text
If the user asks to edit the currently open new item and the current context says it has an empty-path local draft, use current-editor tools if available. Only use path: "" with generic draft tools when the tool explicitly says it targets the active current editor.
```

## Recommendation

Use current-editor tools or a current-editor registry for the immediate global chat UX. In parallel, investigate path-keyed new drafts plus a last-new pointer as a longer-term cleanup.

Avoid making the LLM responsible for interpreting `path: ""` as current. That interpretation belongs in a tool or registry that can verify the active editor.
