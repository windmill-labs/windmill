# Global AI UserDraft Plan

This PR is limited to the generic `UserDraft` primitives and this plan. The
global-mode adapter, prompt copy, and editor-route changes described below
belong in follow-up commits/PRs.

Global mode AI chat should use the same frontend draft model as the user. A
change made through global mode is a user local draft, not a separate AI-owned
draft.

## Goals

- Remove the separate global AI draft store.
- Store global-mode edits in `UserDraft` under the same workspace, item kind,
  and storage path used by editors.
- Keep `UserDraft` domain-agnostic. Mapping between workspace item semantics
  and `UserDraft` storage identities belongs in the global chat adapter.
- Preserve existing editor behavior, including clean-baseline handling,
  staleness checks, autosave debounce, `remove`, and `discard`.

## UserDraft Additions

This PR adds small generic primitives to `UserDraft`:

- Enumerate draft entries for a workspace and bounded item-kind set.
- Expose storage identity: workspace, item kind, storage path.
- Expose whether the value exists in localStorage, a live handle, or both.
- Write a draft value and freshness metadata atomically.
- Clear a draft from both localStorage and any live handle without re-persisting
  the cleared live value.

These primitives must not know about scripts, flows, apps, resources,
variables, triggers, deployment, or chat tools.

## Follow-Up Global Adapter

In the global-mode PR, replace the current global draft store with a thin
adapter around `UserDraft`. The adapter maps global workspace item types to
`UserDraft` identities and value shapes:

- `script` -> `UserDraft('script', storagePath)`
- `flow` -> `UserDraft('flow', storagePath)`
- classic app -> `UserDraft('app', storagePath)`
- raw app -> `UserDraft('raw_app', storagePath)`
- schedule -> `UserDraft('trigger_schedule', storagePath)`
- trigger -> `UserDraft('trigger_<kind>', storagePath)`
- resource -> `UserDraft('resource', storagePath)`
- variable -> `UserDraft('variable', storagePath)`

The adapter is responsible for converting between global tool payloads and
editor payloads, determining whether a local value is meaningful draft work,
and seeding freshness metadata from backend state.

## Existing Items

For existing workspace items, global mode writes to the path-addressed
`UserDraft` key for that item. Before the first write, the adapter fetches the
current backend freshness data and stores it with the draft:

- `remoteRev` for the deployed/current item revision.
- `remoteDraftRev` for backend DB drafts when the item kind supports them.

Subsequent writes preserve this metadata until global mode intentionally
refreshes it.

## New Items

Global-created new items use path-addressed drafts. The storage path is the
intended final workspace path:

- new script `f/tools/foo` -> `UserDraft('script', 'f/tools/foo')`
- new flow `f/tools/bar` -> `UserDraft('flow', 'f/tools/bar')`
- new raw app `f/apps/demo` -> `UserDraft('raw_app', 'f/apps/demo')`

This allows multiple new drafts of the same kind in one workspace. Global mode
must ask for or infer a final path before creating one; it should not create
anonymous multi-drafts.

If a backend item appears at that path after the local draft is created, global
mode must treat deploy/open as a collision or staleness case and ask before
overwriting.

## Add-Page Scratch Drafts

Existing add pages keep their current empty-path scratch drafts:

- `/scripts/add` -> `UserDraft('script', '')`
- `/flows/add` -> `UserDraft('flow', '')`
- `/apps/add` -> `UserDraft('app', '')`
- `/apps_raw/add` -> `UserDraft('raw_app', '')`

Global mode uses these only when acting on the currently open add editor, for
example when the user says "change this script" while `/scripts/add` is active.
In that case the active editor context points global mode at the exact storage
identity, including `storagePath: ''`.

If an empty-path scratch draft and a path-addressed draft both contain the same
intended final item path, global mode should not auto-merge them. It should
list both with their storage identities and ask which one to act on.

## Editor Opening

Edit routes should support path-addressed local-only drafts:

1. Try to load the backend item.
2. If the backend item is missing and `UserDraft.has(kind, path)` is true, load
   the local draft as a new item draft.
3. Save/deploy should create the backend item and clear the matching
   path-addressed `UserDraft` entry.

This is separate from add-page scratch drafts, which remain unchanged.

## Apps

Global mode can create, read, update, list, open, and clear app drafts. Chat
deployment for apps is not required because app save/deploy still goes through
the editor bundling path.

## Debounce

Keep the existing `UserDraft` debounce. Global writes must be immediately
visible through `UserDraft.get`, live handles, enumeration, and global read/list
flows. Direct raw localStorage visibility may lag when a live handle owns the
entry.

## Prompt Copy

Model-facing and user-facing global chat copy should say "local draft" or "your
draft", not "AI draft" or "AI draft store".

## Tests

Split coverage by layer:

- `UserDraft` tests: enumeration, persisted-only drafts, live-only drafts,
  live+persisted dedupe, storage identity, persistence state, clear
  live+persisted, metadata preservation, workspace isolation, kind isolation,
  and serialization-tolerant enumeration.
- Global adapter tests: workspace item mapping, schedule/trigger item-kind
  mapping, path-addressed new drafts, empty-path active editor drafts,
  meaningful-vs-clean filtering, metadata seeding, backend collision detection,
  app deploy unsupported, and prompt/tool copy.
- Editor integration tests where behavior changes: path-addressed local-only
  new scripts/flows/apps can be opened and saved without breaking add-page
  scratch drafts.
